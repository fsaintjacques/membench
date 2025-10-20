fn main() {
    #[cfg(feature = "ebpf")]
    {
        use std::env;
        use std::path::PathBuf;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        // Copy pre-compiled eBPF bytecode to output directory
        // The bytecode is committed to the repository at ebpf/programs.bpf
        // To rebuild: cd ebpf && ./rebuild.sh
        let src = PathBuf::from("ebpf/programs.bpf");
        let dst = out_dir.join("programs");

        std::fs::copy(&src, &dst)
            .expect("failed to copy eBPF bytecode - run 'cd ebpf && ./rebuild.sh' to generate it");

        println!("cargo:rustc-env=EBPF_OUT_DIR={}", out_dir.display());
        println!("cargo:rerun-if-changed=ebpf/programs.bpf");
        println!("cargo:rerun-if-changed=ebpf/src");
    }
}
