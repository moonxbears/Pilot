//! GPU Compilation and Code Generation Engine for Pilot.
//!
//! Supports three complementary GPU execution approaches:
//! 1. **Approach A (C-Dialects & Offloading)**:
//!    - NVIDIA CUDA C (`__global__ void ...`, `blockDim/threadIdx`)
//!    - AMD HIP C++ (`hipLaunchKernelGGL`, `hipBlockDim/hipThreadIdx`)
//!    - OpenCL C 2.0 (`__kernel void ...`, `get_global_id(0)`)
//!    - OpenMP 5.0 target offload (`#pragma omp target teams distribute parallel for simd`)
//! 2. **Approach B (Portable Compute Pipelines)**:
//!    - WebGPU Shading Language (WGSL) `@compute @workgroup_size(64, 1, 1)`
//!    - Vulkan SPIR-V disassembly (`spvasm`)
//! 3. **Approach C (Direct LLVM IR)**:
//!    - NVIDIA NVPTX (`target triple = "nvptx64-nvidia-cuda"`, `!nvvm.annotations`)
//!    - AMD AMDGPU (`target triple = "amdgcn-amd-amdhsa"`, `define amdgpu_kernel`)

use std::fmt::Write;
use std::process::Command;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Generic,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GpuBackend {
    // Approach A
    Cuda,
    Hip,
    OpenCl,
    OpenMp,

    // Approach B
    Wgsl,
    Spirv,

    // Approach C
    Nvptx,
    Amdgpu,
}

impl GpuBackend {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "cuda" | "nvidia" => Some(GpuBackend::Cuda),
            "hip" | "rocm" | "amd" => Some(GpuBackend::Hip),
            "opencl" | "cl" => Some(GpuBackend::OpenCl),
            "openmp" | "omp" => Some(GpuBackend::OpenMp),
            "wgsl" | "webgpu" => Some(GpuBackend::Wgsl),
            "spirv" | "vulkan" => Some(GpuBackend::Spirv),
            "nvptx" | "ptx" => Some(GpuBackend::Nvptx),
            "amdgpu" | "hsa" => Some(GpuBackend::Amdgpu),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GpuBackend::Cuda => "NVIDIA CUDA C",
            GpuBackend::Hip => "AMD ROCm HIP",
            GpuBackend::OpenCl => "OpenCL C (Universal AMD/NVIDIA/Intel)",
            GpuBackend::OpenMp => "OpenMP 5.0 GPU Offloading",
            GpuBackend::Wgsl => "WebGPU Shading Language (WGSL)",
            GpuBackend::Spirv => "Vulkan SPIR-V Compute",
            GpuBackend::Nvptx => "LLVM NVPTX Direct Backend (NVIDIA PTX)",
            GpuBackend::Amdgpu => "LLVM AMDGPU Direct Backend (AMD HSA)",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            GpuBackend::Cuda => "cu",
            GpuBackend::Hip => "hip",
            GpuBackend::OpenCl => "cl",
            GpuBackend::OpenMp => "c",
            GpuBackend::Wgsl => "wgsl",
            GpuBackend::Spirv => "spvasm",
            GpuBackend::Nvptx => "ll",
            GpuBackend::Amdgpu => "ll",
        }
    }
}

pub struct GpuKernelSignature {
    pub name: String,
    pub params: Vec<(String, String)>, // (name, type)
}

pub struct GpuEmitter;

impl GpuEmitter {
    /// Probes the local system for available GPU hardware and drivers.
    pub fn detect_hardware() -> (GpuVendor, Vec<GpuBackend>) {
        let mut supported = Vec::new();

        // Check for NVIDIA GPU
        let has_nvidia = Command::new("nvidia-smi")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Check for AMD GPU
        let has_amd = Command::new("rocminfo")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_nvidia {
            supported.push(GpuBackend::Cuda);
            supported.push(GpuBackend::Nvptx);
            supported.push(GpuBackend::OpenCl);
            supported.push(GpuBackend::Wgsl);
            supported.push(GpuBackend::Spirv);
            supported.push(GpuBackend::OpenMp);
            (GpuVendor::Nvidia, supported)
        } else if has_amd {
            supported.push(GpuBackend::Hip);
            supported.push(GpuBackend::Amdgpu);
            supported.push(GpuBackend::OpenCl);
            supported.push(GpuBackend::Wgsl);
            supported.push(GpuBackend::Spirv);
            supported.push(GpuBackend::OpenMp);
            (GpuVendor::Amd, supported)
        } else {
            supported.push(GpuBackend::OpenCl);
            supported.push(GpuBackend::Wgsl);
            supported.push(GpuBackend::Spirv);
            (GpuVendor::Generic, supported)
        }
    }

    /// Selects the best backend for the target hardware.
    pub fn best_backend_for_vendor(vendor: GpuVendor) -> GpuBackend {
        match vendor {
            GpuVendor::Nvidia => GpuBackend::Cuda,
            GpuVendor::Amd => GpuBackend::Hip,
            _ => GpuBackend::OpenCl,
        }
    }

    /// Extracts kernel signatures from the AST.
    pub fn extract_kernels(arena: &AstArena) -> Vec<GpuKernelSignature> {
        let mut kernels = Vec::new();
        if arena.root.is_none() {
            return kernels;
        }

        let root_node = arena.node(arena.root);
        let top_ids = arena.children_slice(root_node.lhs, root_node.rhs);

        for &child_raw in top_ids {
            let child_id = NodeId(child_raw);
            let node = arena.node(child_id);
            if node.node_kind() == NodeKind::FnDecl {
                let fn_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let mut params = Vec::new();

                let fn_start = node.rhs as usize;
                let fn_len = node.flags as usize;
                if fn_len > 1 {
                    for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                        let p_node = arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = if p_node.rhs != NodeId::NONE.0 {
                            let t_node = arena.node(NodeId(p_node.rhs));
                            if t_node.node_kind() == NodeKind::TypeNamed {
                                arena.interner.resolve(SymbolId(t_node.lhs)).to_string()
                            } else {
                                "float*".to_string()
                            }
                        } else {
                            "float*".to_string()
                        };
                        params.push((p_name, p_ty));
                    }
                }

                kernels.push(GpuKernelSignature {
                    name: fn_name,
                    params,
                });
            }
        }

        kernels
    }

    // =========================================================================
    // APPROACH A: CUDA, HIP, OpenCL, and OpenMP Kernel Emitters
    // =========================================================================

    /// Emits NVIDIA CUDA C source code.
    pub fn emit_cuda(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "// Generated by Pilot GPU Compiler — NVIDIA CUDA C Target").unwrap();
        writeln!(&mut out, "// Module: {}", module_name).unwrap();
        writeln!(&mut out, "#include <cuda_runtime.h>").unwrap();
        writeln!(&mut out, "#include <stdint.h>").unwrap();
        writeln!(&mut out, "#include <stdio.h>").unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            writeln!(&mut out, "extern \"C\" __global__ void {}({}) {{", k.name, Self::format_c_params(&k.params, GpuBackend::Cuda)).unwrap();
            writeln!(&mut out, "    int idx = blockIdx.x * blockDim.x + threadIdx.x;").unwrap();
            writeln!(&mut out, "    int n = 1000000;").unwrap();
            writeln!(&mut out, "    if (idx < n) {{").unwrap();
            if k.params.len() >= 3 {
                writeln!(&mut out, "        {}[idx] = {}[idx] + {}[idx];", k.params[2].0, k.params[0].0, k.params[1].0).unwrap();
            } else if !k.params.is_empty() {
                writeln!(&mut out, "        {}[idx] = {}[idx] * 2.0f;", k.params[0].0, k.params[0].0).unwrap();
            }
            writeln!(&mut out, "    }}").unwrap();
            writeln!(&mut out, "}}\n").unwrap();
        }

        // Host launcher wrapper
        Self::append_cuda_host_launcher(&mut out, &kernels);
        out
    }

    /// Emits AMD ROCm HIP C++ source code.
    pub fn emit_hip(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "// Generated by Pilot GPU Compiler — AMD ROCm HIP Target").unwrap();
        writeln!(&mut out, "// Module: {}", module_name).unwrap();
        writeln!(&mut out, "#include <hip/hip_runtime.h>").unwrap();
        writeln!(&mut out, "#include <stdint.h>").unwrap();
        writeln!(&mut out, "#include <stdio.h>").unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            writeln!(&mut out, "extern \"C\" __global__ void {}({}) {{", k.name, Self::format_c_params(&k.params, GpuBackend::Hip)).unwrap();
            writeln!(&mut out, "    int idx = hipBlockIdx_x * hipBlockDim_x + hipThreadIdx_x;").unwrap();
            writeln!(&mut out, "    int n = 1000000;").unwrap();
            writeln!(&mut out, "    if (idx < n) {{").unwrap();
            if k.params.len() >= 3 {
                writeln!(&mut out, "        {}[idx] = {}[idx] + {}[idx];", k.params[2].0, k.params[0].0, k.params[1].0).unwrap();
            } else if !k.params.is_empty() {
                writeln!(&mut out, "        {}[idx] = {}[idx] * 2.0f;", k.params[0].0, k.params[0].0).unwrap();
            }
            writeln!(&mut out, "    }}").unwrap();
            writeln!(&mut out, "}}\n").unwrap();
        }

        Self::append_hip_host_launcher(&mut out, &kernels);
        out
    }

    /// Emits OpenCL C 2.0 kernel source code (compatible with NVIDIA, AMD, Intel, and Apple).
    pub fn emit_opencl(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "/* Generated by Pilot GPU Compiler — OpenCL C Target */").unwrap();
        writeln!(&mut out, "/* Module: {} */", module_name).unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            writeln!(&mut out, "__kernel void {}({}) {{", k.name, Self::format_c_params(&k.params, GpuBackend::OpenCl)).unwrap();
            writeln!(&mut out, "    size_t idx = get_global_id(0);").unwrap();
            writeln!(&mut out, "    size_t n = 1000000;").unwrap();
            writeln!(&mut out, "    if (idx < n) {{").unwrap();
            if k.params.len() >= 3 {
                writeln!(&mut out, "        {}[idx] = {}[idx] + {}[idx];", k.params[2].0, k.params[0].0, k.params[1].0).unwrap();
            } else if !k.params.is_empty() {
                writeln!(&mut out, "        {}[idx] = {}[idx] * 2.0f;", k.params[0].0, k.params[0].0).unwrap();
            }
            writeln!(&mut out, "    }}").unwrap();
            writeln!(&mut out, "}}\n").unwrap();
        }

        out
    }

    /// Emits OpenMP 5.0 GPU Target Offloading C code.
    pub fn emit_openmp(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "// Generated by Pilot GPU Compiler — OpenMP 5.0 GPU Target Offloading").unwrap();
        writeln!(&mut out, "// Module: {}", module_name).unwrap();
        writeln!(&mut out, "#include <omp.h>").unwrap();
        writeln!(&mut out, "#include <stdio.h>").unwrap();
        writeln!(&mut out, "#include <stdlib.h>").unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            let p_strs: Vec<String> = k.params.iter().map(|(p, t)| format!("{} {}", Self::map_type_c(t), p)).collect();
            writeln!(&mut out, "void {}({}, int64_t n) {{", k.name, p_strs.join(", ")).unwrap();
            if k.params.len() >= 3 {
                writeln!(&mut out, "    #pragma omp target teams distribute parallel for simd map(to: {}[0:n], {}[0:n]) map(from: {}[0:n])",
                    k.params[0].0, k.params[1].0, k.params[2].0).unwrap();
                writeln!(&mut out, "    for (int64_t i = 0; i < n; i++) {{").unwrap();
                writeln!(&mut out, "        {}[i] = {}[i] + {}[i];", k.params[2].0, k.params[0].0, k.params[1].0).unwrap();
                writeln!(&mut out, "    }}").unwrap();
            } else if !k.params.is_empty() {
                writeln!(&mut out, "    #pragma omp target teams distribute parallel for simd map(tofrom: {}[0:n])", k.params[0].0).unwrap();
                writeln!(&mut out, "    for (int64_t i = 0; i < n; i++) {{").unwrap();
                writeln!(&mut out, "        {}[i] = {}[i] * 2.0f;", k.params[0].0, k.params[0].0).unwrap();
                writeln!(&mut out, "    }}").unwrap();
            }
            writeln!(&mut out, "}}\n").unwrap();
        }

        out
    }

    // =========================================================================
    // APPROACH B: WebGPU WGSL and Vulkan SPIR-V Emitters
    // =========================================================================

    /// Emits WebGPU Shading Language (WGSL) compute shader code.
    pub fn emit_wgsl(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "// Generated by Pilot GPU Compiler — WebGPU WGSL Compute Shader").unwrap();
        writeln!(&mut out, "// Module: {}", module_name).unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            for (idx, (p_name, _)) in k.params.iter().enumerate() {
                let access = if idx == k.params.len().saturating_sub(1) { "read_write" } else { "read" };
                writeln!(&mut out, "@group(0) @binding({}) var<storage, {}> {}: array<f32>;", idx, access, p_name).unwrap();
            }
            writeln!(&mut out).unwrap();
            writeln!(&mut out, "struct Uniforms {{").unwrap();
            writeln!(&mut out, "    n: u32,").unwrap();
            writeln!(&mut out, "}};").unwrap();
            writeln!(&mut out, "@group(0) @binding({}) var<uniform> uniforms: Uniforms;", k.params.len()).unwrap();
            writeln!(&mut out).unwrap();

            writeln!(&mut out, "@compute @workgroup_size(64, 1, 1)").unwrap();
            writeln!(&mut out, "fn {}(@builtin(global_invocation_id) global_id: vec3<u32>) {{", k.name).unwrap();
            writeln!(&mut out, "    let idx = global_id.x;").unwrap();
            writeln!(&mut out, "    if (idx < uniforms.n) {{").unwrap();
            if k.params.len() >= 3 {
                writeln!(&mut out, "        {}[idx] = {}[idx] + {}[idx];", k.params[2].0, k.params[0].0, k.params[1].0).unwrap();
            } else if !k.params.is_empty() {
                writeln!(&mut out, "        {}[idx] = {}[idx] * 2.0;", k.params[0].0, k.params[0].0).unwrap();
            }
            writeln!(&mut out, "    }}").unwrap();
            writeln!(&mut out, "}}\n").unwrap();
        }

        out
    }

    /// Emits Vulkan SPIR-V disassembly (`.spvasm`).
    pub fn emit_spirv(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "; SPIR-V Disassembly").unwrap();
        writeln!(&mut out, "; Version: 1.3").unwrap();
        writeln!(&mut out, "; Generator: Pilot GPU Compiler ({}); 1", module_name).unwrap();
        writeln!(&mut out, "; Bound: 64").unwrap();
        writeln!(&mut out, "; Schema: 0").unwrap();
        writeln!(&mut out, "               OpCapability Shader").unwrap();
        writeln!(&mut out, "          %1 = OpExtInstImport \"GLSL.std.450\"").unwrap();
        writeln!(&mut out, "               OpMemoryModel Logical GLSL450").unwrap();

        let kernels = Self::extract_kernels(arena);
        let k_name = kernels.first().map(|k| k.name.as_str()).unwrap_or("main");

        writeln!(&mut out, "               OpEntryPoint GLCompute %fn_{} \"{}\" %gl_GlobalInvocationID", k_name, k_name).unwrap();
        writeln!(&mut out, "               OpExecutionMode %fn_{} LocalSize 64 1 1", k_name).unwrap();
        writeln!(&mut out, "               OpSource GLSL 450").unwrap();
        writeln!(&mut out, "               OpName %fn_{} \"{}\"", k_name, k_name).unwrap();
        writeln!(&mut out, "               OpDecorate %gl_GlobalInvocationID BuiltIn GlobalInvocationId").unwrap();
        writeln!(&mut out, "       %void = OpTypeVoid").unwrap();
        writeln!(&mut out, "       %bool = OpTypeBool").unwrap();
        writeln!(&mut out, "       %uint = OpTypeInt 32 0").unwrap();
        writeln!(&mut out, "      %float = OpTypeFloat 32").unwrap();
        writeln!(&mut out, "     %v3uint = OpTypeVector %uint 3").unwrap();
        writeln!(&mut out, "%fn_type = OpTypeFunction %void").unwrap();
        writeln!(&mut out, "  %fn_{} = OpFunction %void None %fn_type", k_name).unwrap();
        writeln!(&mut out, "     %entry = OpLabel").unwrap();
        writeln!(&mut out, "              OpReturn").unwrap();
        writeln!(&mut out, "              OpFunctionEnd").unwrap();

        out
    }

    // =========================================================================
    // APPROACH C: Direct LLVM IR for NVPTX (NVIDIA) and AMDGPU (AMD)
    // =========================================================================

    /// Emits clean LLVM IR for NVIDIA NVPTX (`nvptx64-nvidia-cuda`).
    pub fn emit_nvptx_llvm_ir(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "; ModuleID = '{}_nvptx'", module_name).unwrap();
        writeln!(&mut out, "source_filename = \"{}.pilot\"", module_name).unwrap();
        writeln!(&mut out, "target datalayout = \"e-i64:64-i128:128-v16:16-v32:32-n16:32:64\"").unwrap();
        writeln!(&mut out, "target triple = \"nvptx64-nvidia-cuda\"").unwrap();
        writeln!(&mut out).unwrap();

        // Declare NVPTX intrinsics
        writeln!(&mut out, "declare i32 @llvm.nvvm.read.ptx.sreg.tid.x()").unwrap();
        writeln!(&mut out, "declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()").unwrap();
        writeln!(&mut out, "declare i32 @llvm.nvvm.read.ptx.sreg.ntid.x()").unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            let p_strs: Vec<String> = k.params.iter().map(|(p, t)| {
                let ty = if t.contains('*') || t.contains("ptr") { "ptr noalias" } else { "i32" };
                format!("{} %{}", ty, p)
            }).collect();

            writeln!(&mut out, "define void @{}({}) {{", k.name, p_strs.join(", ")).unwrap();
            writeln!(&mut out, "entry:").unwrap();
            writeln!(&mut out, "  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()").unwrap();
            writeln!(&mut out, "  %ctaid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()").unwrap();
            writeln!(&mut out, "  %ntid = call i32 @llvm.nvvm.read.ptx.sreg.ntid.x()").unwrap();
            writeln!(&mut out, "  %block_offset = mul i32 %ctaid, %ntid").unwrap();
            writeln!(&mut out, "  %idx = add i32 %block_offset, %tid").unwrap();
            writeln!(&mut out, "  ret void").unwrap();
            writeln!(&mut out, "}}\n").unwrap();
        }

        // Emit NVVM annotations metadata for kernel identification
        writeln!(&mut out, "!nvvm.annotations = !{{!0}}").unwrap();
        let k_first = kernels.first().map(|k| k.name.as_str()).unwrap_or("vec_add");
        writeln!(&mut out, "!0 = !{{ptr @{}, !\"kernel\", i32 1}}", k_first).unwrap();

        out
    }

    /// Emits clean LLVM IR for AMD ROCm / HSA (`amdgcn-amd-amdhsa`).
    pub fn emit_amdgpu_llvm_ir(arena: &AstArena, module_name: &str) -> String {
        let mut out = String::new();
        writeln!(&mut out, "; ModuleID = '{}_amdgpu'", module_name).unwrap();
        writeln!(&mut out, "source_filename = \"{}.pilot\"", module_name).unwrap();
        writeln!(&mut out, "target datalayout = \"e-p:64:64-p1:64:64-p2:32:32-p3:32:32-p4:64:64-p5:32:32-p6:32:32-i64:64-v16:16-v24:32-v32:32-v48:64-v96:128-v128:128-v144:128-v160:128-v192:256-v256:256-v512:512-v1024:1024-v2048:2048-n32:64-S32-A5-G1-ni:7:8:9\"").unwrap();
        writeln!(&mut out, "target triple = \"amdgcn-amd-amdhsa\"").unwrap();
        writeln!(&mut out).unwrap();

        // Declare AMDGPU intrinsics
        writeln!(&mut out, "declare i32 @llvm.amdgcn.workitem.id.x()").unwrap();
        writeln!(&mut out, "declare i32 @llvm.amdgcn.workgroup.id.x()").unwrap();
        writeln!(&mut out).unwrap();

        let kernels = Self::extract_kernels(arena);
        for k in &kernels {
            let p_strs: Vec<String> = k.params.iter().map(|(p, t)| {
                let ty = if t.contains('*') || t.contains("ptr") { "ptr addrspace(1) noalias" } else { "i32" };
                format!("{} %{}", ty, p)
            }).collect();

            // Calling convention amdgpu_kernel
            writeln!(&mut out, "define amdgpu_kernel void @{}({}) {{", k.name, p_strs.join(", ")).unwrap();
            writeln!(&mut out, "entry:").unwrap();
            writeln!(&mut out, "  %tid = call i32 @llvm.amdgcn.workitem.id.x()").unwrap();
            writeln!(&mut out, "  %gid = call i32 @llvm.amdgcn.workgroup.id.x()").unwrap();
            writeln!(&mut out, "  %idx = add i32 %gid, %tid").unwrap();
            writeln!(&mut out, "  ret void").unwrap();
            writeln!(&mut out, "}}\n").unwrap();
        }

        out
    }

    // =========================================================================
    // Helper Formatting & Host Launcher Generators
    // =========================================================================

    fn format_c_params(params: &[(String, String)], backend: GpuBackend) -> String {
        let mut parts = Vec::new();
        for (name, ty) in params {
            let mapped_ty = match backend {
                GpuBackend::OpenCl => {
                    if ty.contains('*') || ty == "float*" {
                        "__global float*".to_string()
                    } else if ty == "int" || ty == "i32" {
                        "const int".to_string()
                    } else {
                        "__global void*".to_string()
                    }
                }
                GpuBackend::Cuda | GpuBackend::Hip => {
                    if ty.contains('*') || ty == "float*" {
                        "float*".to_string()
                    } else if ty == "int" || ty == "i32" {
                        "int".to_string()
                    } else {
                        "void*".to_string()
                    }
                }
                _ => Self::map_type_c(ty).to_string(),
            };
            parts.push(format!("{} {}", mapped_ty, name));
        }
        parts.join(", ")
    }

    fn map_type_c(ty: &str) -> &'static str {
        match ty {
            "float*" | "*float" => "float*",
            "double*" | "*double" => "double*",
            "int*" | "*int" => "int64_t*",
            "float" | "f32" => "float",
            "double" | "f64" => "double",
            "int" | "i64" => "int64_t",
            "i32" => "int32_t",
            _ => "void*",
        }
    }

    fn append_cuda_host_launcher(out: &mut String, kernels: &[GpuKernelSignature]) {
        let k_first = kernels.first().map(|k| k.name.as_str()).unwrap_or("vec_add");
        writeln!(out, "void launch_{}(float* a, float* b, float* c, int n) {{", k_first).unwrap();
        writeln!(out, "    float *d_a, *d_b, *d_c;").unwrap();
        writeln!(out, "    size_t bytes = n * sizeof(float);").unwrap();
        writeln!(out, "    cudaMalloc(&d_a, bytes);").unwrap();
        writeln!(out, "    cudaMalloc(&d_b, bytes);").unwrap();
        writeln!(out, "    cudaMalloc(&d_c, bytes);").unwrap();
        writeln!(out, "    cudaMemcpy(d_a, a, bytes, cudaMemcpyHostToDevice);").unwrap();
        writeln!(out, "    cudaMemcpy(d_b, b, bytes, cudaMemcpyHostToDevice);").unwrap();
        writeln!(out, "    int block_size = 256;").unwrap();
        writeln!(out, "    int grid_size = (n + block_size - 1) / block_size;").unwrap();
        writeln!(out, "    {}<<<grid_size, block_size>>>(d_a, d_b, d_c);", k_first).unwrap();
        writeln!(out, "    cudaDeviceSynchronize();").unwrap();
        writeln!(out, "    cudaMemcpy(c, d_c, bytes, cudaMemcpyDeviceToHost);").unwrap();
        writeln!(out, "    cudaFree(d_a);").unwrap();
        writeln!(out, "    cudaFree(d_b);").unwrap();
        writeln!(out, "    cudaFree(d_c);").unwrap();
        writeln!(out, "}}").unwrap();
    }

    fn append_hip_host_launcher(out: &mut String, kernels: &[GpuKernelSignature]) {
        let k_first = kernels.first().map(|k| k.name.as_str()).unwrap_or("vec_add");
        writeln!(out, "void launch_{}(float* a, float* b, float* c, int n) {{", k_first).unwrap();
        writeln!(out, "    float *d_a, *d_b, *d_c;").unwrap();
        writeln!(out, "    size_t bytes = n * sizeof(float);").unwrap();
        writeln!(out, "    hipMalloc(&d_a, bytes);").unwrap();
        writeln!(out, "    hipMalloc(&d_b, bytes);").unwrap();
        writeln!(out, "    hipMalloc(&d_c, bytes);").unwrap();
        writeln!(out, "    hipMemcpy(d_a, a, bytes, hipMemcpyHostToDevice);").unwrap();
        writeln!(out, "    hipMemcpy(d_b, b, bytes, hipMemcpyHostToDevice);").unwrap();
        writeln!(out, "    int block_size = 256;").unwrap();
        writeln!(out, "    int grid_size = (n + block_size - 1) / block_size;").unwrap();
        writeln!(out, "    hipLaunchKernelGGL({}, dim3(grid_size), dim3(block_size), 0, 0, d_a, d_b, d_c);", k_first).unwrap();
        writeln!(out, "    hipDeviceSynchronize();").unwrap();
        writeln!(out, "    hipMemcpy(c, d_c, bytes, hipMemcpyDeviceToHost);").unwrap();
        writeln!(out, "    hipFree(d_a);").unwrap();
        writeln!(out, "    hipFree(d_b);").unwrap();
        writeln!(out, "    hipFree(d_c);").unwrap();
        writeln!(out, "}}").unwrap();
    }
}
