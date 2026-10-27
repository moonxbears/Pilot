	.text
	.file	"01_burst_vs_vm_benchmark.pilot"
	# Pilot Burst Native Compilation Engine (LLVM-AVX2 Mode)
	# Optimization Level: -O3 | Burst Vectorization: Enabled (AVX2/FMA)
	# Architecture: x86_64 | Calling Convention: System V / Microsoft x64

	.globl	update_particles
	.def	update_particles; .scl 2; .type 32; .endef
	.p2align	4, 0x90
update_particles:
	.seh_proc	update_particles
	# --- Begin Function: update_particles ---
	pushq	%rbp
	.seh_pushreg	%rbp
	movq	%rsp, %rbp
	.seh_setframe	%rbp, 0
	subq	$64, %rsp

.L01_burst_vs_vm_benchmark_entry:
	jmp	.L01_burst_vs_vm_benchmark_ %while.cond.0
.L01_burst_vs_vm_benchmark_while.cond.0:
	testb	%al, %al
	jne	.L01_burst_vs_vm_benchmark_loop.body
	jmp	.L01_burst_vs_vm_benchmark_loop.end
.L01_burst_vs_vm_benchmark_while.body.1:
	jmp	.L01_burst_vs_vm_benchmark_ %while.cond.3
.L01_burst_vs_vm_benchmark_while.cond.3:
	testb	%al, %al
	jne	.L01_burst_vs_vm_benchmark_loop.body
	jmp	.L01_burst_vs_vm_benchmark_loop.end
.L01_burst_vs_vm_benchmark_while.body.4:
	vaddpd	%xmm1, %xmm0, %xmm0	# AVX vector add
	jmp	.L01_burst_vs_vm_benchmark_ %while.cond.3
.L01_burst_vs_vm_benchmark_while.end.5:
	vaddpd	%xmm1, %xmm0, %xmm0	# AVX vector add
	jmp	.L01_burst_vs_vm_benchmark_ %while.cond.0
.L01_burst_vs_vm_benchmark_while.end.2:
	# Epilogue
	addq	$64, %rsp
	popq	%rbp
	retq
	.seh_endproc
	# --- End Function: update_particles ---

	.globl	main
	.def	main; .scl 2; .type 32; .endef
	.p2align	4, 0x90
main:
	.seh_proc	main
	# --- Begin Function: main ---
	pushq	%rbp
	.seh_pushreg	%rbp
	movq	%rsp, %rbp
	.seh_setframe	%rbp, 0
	subq	$64, %rsp

.L01_burst_vs_vm_benchmark_entry:
	jmp	.L01_burst_vs_vm_benchmark_ %while.cond.0
.L01_burst_vs_vm_benchmark_while.cond.0:
	testb	%al, %al
	jne	.L01_burst_vs_vm_benchmark_loop.body
	jmp	.L01_burst_vs_vm_benchmark_loop.end
.L01_burst_vs_vm_benchmark_while.body.1:
	vmulpd	%xmm1, %xmm0, %xmm0	# AVX vector multiply
	vaddpd	%xmm1, %xmm0, %xmm0	# AVX vector add
	jmp	.L01_burst_vs_vm_benchmark_ %while.cond.0
.L01_burst_vs_vm_benchmark_while.end.2:
	# Epilogue
	addq	$64, %rsp
	popq	%rbp
	retq
	.seh_endproc
	# --- End Function: main ---

	.ident	"Pilot Compiler v1.1.0 (Burst/AVX2 Engine)"
	.section	.note.GNU-stack,"",@progbits
