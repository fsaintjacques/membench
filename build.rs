use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    #[cfg(feature = "ebpf")]
    {
        use std::path::PathBuf;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        // Compile eBPF program
        let status = Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--target=bpfel-unknown-none",
                "-Z", "build-std=core",
            ])
            .current_dir("ebpf")
            .status();

        match status {
            Ok(s) if s.success() => {
                // Copy compiled program to output directory
                let src = PathBuf::from("ebpf/target/bpfel-unknown-none/release/programs");
                let dst = out_dir.join("programs");
                std::fs::copy(&src, &dst)
                    .expect("failed to copy eBPF program");

                println!("cargo:warning=eBPF program compiled successfully");
            }
            _ => {
                println!("cargo:warning=eBPF program compilation failed - stub used");
                // Create empty file as fallback
                std::fs::write(out_dir.join("programs"), b"")
                    .expect("failed to create stub");
            }
        }

        println!("cargo:rustc-env=EBPF_OUT_DIR={}", out_dir.display());
        println!("cargo:rerun-if-changed=ebpf/src");
    }
}
