//! Milestone 35.1: Unified Native AOT Build Driver (pilot build).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use crate::asm::AsmEmitter;
use crate::c_target::CTargetEmitter;
use crate::llvm::LlvmEmitter;
use crate::wasm::WasmEmitter;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BuildTarget {
    Native,
    Llvm,
    Wasm,
    C,
    Asm,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O2,
    O3,
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub target: BuildTarget,
    pub opt_level: OptLevel,
    pub output_path: PathBuf,
    pub pgo_trace: Option<PathBuf>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            target: BuildTarget::Native,
            opt_level: OptLevel::O2,
            output_path: PathBuf::from("a.out"),
            pgo_trace: None,
        }
    }
}

pub struct BuildDriver;

impl BuildDriver {
    pub fn compile_source_to_artifact(
        module_name: &str,
        options: &BuildOptions,
    ) -> Result<String, String> {
        Self::compile_pilot_source(module_name, "", options)
    }

    pub fn compile_pilot_source(
        module_name: &str,
        source_text: &str,
        options: &BuildOptions,
    ) -> Result<String, String> {
        let arena = pilot_parser::parse(source_text);
        let root = arena.root;
        let analysis = pilot_analysis::pipeline::AnalysisPipeline::analyze(&arena, root);

        match options.target {
            BuildTarget::C => {
                let header_content = pilot_rt::generate_c_runtime_header();
                let _ = fs::write("/tmp/pilot_runtime.h", header_content);
                let _ = fs::write("pilot_runtime.h", header_content);
                let c_code = CTargetEmitter::emit_ast_module(&arena, &analysis, source_text);
                Ok(c_code)
            }
            BuildTarget::Llvm => {
                let mut emitter = LlvmEmitter::new();
                Ok(emitter.emit_arena_program(&arena, module_name))
            }
            BuildTarget::Wasm => {
                let mut emitter = WasmEmitter::new();
                Ok(emitter.emit_wat_from_ast(&arena, module_name))
            }
            BuildTarget::Asm => {
                let opt = match options.opt_level {
                    OptLevel::O0 => 0,
                    OptLevel::O2 => 2,
                    OptLevel::O3 => 3,
                };
                AsmEmitter::emit_assembly(&arena, module_name, opt, true)
            }
            BuildTarget::Native => {
                let c_code = CTargetEmitter::emit_ast_module(&arena, &analysis, source_text);
                let opt_flag = match options.opt_level {
                    OptLevel::O0 => "-O0",
                    OptLevel::O2 => "-O2",
                    OptLevel::O3 => "-O3",
                };

                let tmp_dir = std::env::temp_dir().join(format!("pilot_build_{}", std::process::id()));
                let _ = fs::create_dir_all(&tmp_dir);
                let tmp_c = tmp_dir.join(format!("{}.c", module_name));
                let header_path = tmp_dir.join("pilot_runtime.h");

                // Write out C-ABI runtime header
                let header_content = pilot_rt::generate_c_runtime_header();
                let _ = fs::write("/tmp/pilot_runtime.h", header_content);
                let _ = fs::write("pilot_runtime.h", header_content);
                if let Err(e) = fs::write(&header_path, header_content) {
                    let _ = fs::remove_dir_all(&tmp_dir);
                    return Err(format!("Failed to write runtime header: {}", e));
                }

                if let Err(e) = fs::write(&tmp_c, &c_code) {
                    let _ = fs::remove_dir_all(&tmp_dir);
                    return Err(format!("Failed to write temporary C source: {}", e));
                }

                let compiler = if Command::new("clang").arg("--version").output().is_ok() {
                    "clang"
                } else {
                    "gcc"
                };

                let inc_flag = format!("-I{}", tmp_dir.display());
                let mut cmd = Command::new(compiler);
                cmd.arg(opt_flag)
                    .arg("-fno-strict-aliasing")
                    .arg("-fomit-frame-pointer")
                    .arg("-fno-math-errno");

                if options.opt_level == OptLevel::O3 {
                    cmd.arg("-march=native");
                }

                let status = cmd
                    .args([
                        &inc_flag,
                        tmp_c.to_str().unwrap(),
                        "-lm",
                        "-pthread",
                        "-o",
                        options.output_path.to_str().unwrap_or("a.out"),
                    ])
                    .status();

                let _ = fs::remove_dir_all(&tmp_dir);

                match status {
                    Ok(s) if s.success() => Ok(format!("Compiled native binary -> {:?}", options.output_path)),
                    Ok(s) => Err(format!("C compiler failed with exit code: {}", s)),
                    Err(e) => Err(format!("Failed to spawn C compiler: {}", e)),
                }
            }
        }
    }
}
