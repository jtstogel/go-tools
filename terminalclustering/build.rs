// build.rs
use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let katago_dir = manifest.join("submodules/KataGo");
    let patch_file = manifest.join("submodules/patches/KataGo.patch");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Rebuild when the submodule or patch changes
    println!("cargo:rerun-if-changed={}", katago_dir.display());
    println!("cargo:rerun-if-changed={}", patch_file.display());

    // Sanity checks (keep build.rs offline)
    if !katago_dir.join("cpp/CMakeLists.txt").exists() {
        panic!("KataGo submodule missing. Run: `git submodule update --init --recursive`");
    }
    if !patch_file.exists() {
        panic!("Patch file not found at {}", patch_file.display());
    }

    // Apply the patch once per target dir
    let applied_stamp = out_dir.join("katago_patch_applied.stamp");
    if !applied_stamp.exists() {
        apply_patch(&katago_dir, &patch_file);
        fs::write(&applied_stamp, b"ok").unwrap();
    }

    // Configure & build with CMake (tweak options for CUDA/OpenCL)
    let build_dir = out_dir.join("katago-build");
    fs::create_dir_all(&build_dir).unwrap();

    run(
        Command::new("cmake")
            .current_dir(&build_dir)
            .arg(katago_dir.join("cpp").as_os_str())
            .arg("-DBUILD_DISTRIBUTED=0")
            .arg("-DUSE_BACKEND=CUDA"),
    );

    run(
        Command::new("make")
            .current_dir(&build_dir)
            .arg("-j24")
    );

    // If you link against a produced lib/binary, emit cargo:rustc-link-* here.
}

fn apply_patch(src: &PathBuf, patch: &PathBuf) {
    // Try POSIX patch(1) first (idempotent via --forward), then fall back to git apply --3way
    let cmd = format!(
        "cd '{}' && patch -p1 --forward --reject-file=- < '{}'",
        src.display(),
        patch.display()
    );
    let ok = Command::new("sh").arg("-c").arg(&cmd).status().map(|s| s.success()).unwrap_or(false);

    if !ok {
        // Windows-friendly fallback (requires Git in PATH)
        let cmd = format!(
            "cd '{}' && git apply --3way --ignore-space-change --whitespace=nowarn '{}'",
            src.display(),
            patch.display()
        );
        let status = Command::new("sh").arg("-c").arg(&cmd).status()
            .expect("failed to execute git apply");
        if !status.success() {
            panic!("Failed applying patch: {}", patch.display());
        }
    }
}

fn run(cmd: &mut Command) {
    let status = cmd.status().expect("failed to run command");
    if !status.success() {
        panic!("command failed: {:?}", cmd);
    }
}
