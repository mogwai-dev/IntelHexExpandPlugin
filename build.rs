use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn ensure_x64_target() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch != "x86_64" {
        panic!("IntelHexExpandPlugin is x64-only. Build with target x86_64-pc-windows-msvc.");
    }
}

fn find_midl() -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(sdk_dir) = env::var("WindowsSdkDir") {
        roots.push(PathBuf::from(sdk_dir));
    }
    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files_x86).join("Windows Kits").join("10"));
    }

    for root in roots {
        let bin_dir = root.join("bin");
        if !bin_dir.is_dir() {
            continue;
        }

        let mut candidates = Vec::new();
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let version_dir = entry.path();
                if !version_dir.is_dir() {
                    continue;
                }

                let midl = version_dir.join("x64").join("midl.exe");
                if midl.is_file() {
                    candidates.push(midl);
                }
            }
        }

        candidates.sort();
        if let Some(midl) = candidates.pop() {
            return Some(midl);
        }
    }

    None
}

fn find_cl_x64() -> Option<PathBuf> {
    if let Ok(vc_tools_dir) = env::var("VCToolsInstallDir") {
        let cl = PathBuf::from(vc_tools_dir)
            .join("bin")
            .join("Hostx64")
            .join("x64")
            .join("cl.exe");
        if cl.is_file() {
            return Some(cl);
        }
    }

    let mut roots = Vec::new();
    for env_var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(program_files) = env::var(env_var) {
            roots.push(
                PathBuf::from(&program_files)
                    .join("Microsoft Visual Studio")
                    .join("2022"),
            );
            roots.push(
                PathBuf::from(&program_files)
                    .join("Microsoft Visual Studio")
                    .join("2019"),
            );
        }
    }

    for root in roots {
        if !root.is_dir() {
            continue;
        }

        let mut candidates = Vec::new();
        if let Ok(editions) = fs::read_dir(&root) {
            for edition in editions.flatten() {
                let msvc_root = edition.path().join("VC").join("Tools").join("MSVC");
                if !msvc_root.is_dir() {
                    continue;
                }

                if let Ok(versions) = fs::read_dir(msvc_root) {
                    for version in versions.flatten() {
                        let cl = version
                            .path()
                            .join("bin")
                            .join("Hostx64")
                            .join("x64")
                            .join("cl.exe");
                        if cl.is_file() {
                            candidates.push(cl);
                        }
                    }
                }
            }
        }

        candidates.sort();
        if let Some(cl) = candidates.pop() {
            return Some(cl);
        }
    }

    None
}

fn find_windows_sdk_include_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(sdk_dir) = env::var("WindowsSdkDir") {
        roots.push(PathBuf::from(sdk_dir));
    }
    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files_x86).join("Windows Kits").join("10"));
    }

    for root in roots {
        let include_root = root.join("Include");
        if !include_root.is_dir() {
            continue;
        }

        let mut versions = Vec::new();
        if let Ok(entries) = fs::read_dir(&include_root) {
            for entry in entries.flatten() {
                let dir = entry.path();
                if dir.is_dir() {
                    versions.push(dir);
                }
            }
        }

        versions.sort();
        if let Some(version_dir) = versions.pop() {
            let mut include_dirs = Vec::new();
            for name in ["um", "shared", "winrt", "ucrt"] {
                let dir = version_dir.join(name);
                if dir.is_dir() {
                    include_dirs.push(dir);
                }
            }
            if !include_dirs.is_empty() {
                return include_dirs;
            }
        }
    }

    Vec::new()
}

fn run_midl(idl_path: &Path, out_dir: &Path) -> PathBuf {
    let tlb_path = out_dir.join("IntelHexExpand.tlb");
    let include_dirs = find_windows_sdk_include_dirs();

    let mut midl_cmd = Command::new("midl");
    if let Some(midl_path) = find_midl() {
        midl_cmd = Command::new(midl_path);
    }

    if let Some(cl_path) = find_cl_x64() {
        if let Some(cl_dir) = cl_path.parent() {
            let path_env = env::var("PATH").unwrap_or_default();
            midl_cmd.env("PATH", format!("{};{}", cl_dir.display(), path_env));
        }

        if !include_dirs.is_empty() {
            let include_env = include_dirs
                .iter()
                .map(|dir| dir.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(";");
            midl_cmd.env("INCLUDE", include_env);
        }

        midl_cmd.arg("/cpp_cmd").arg("cl.exe");
        midl_cmd.arg("/cpp_opt").arg("/nologo /EP");
    }

    for include_dir in &include_dirs {
        midl_cmd.arg("/I").arg(include_dir);
    }

    let status = midl_cmd
        .arg(idl_path)
        .arg("/out")
        .arg(out_dir)
        .arg("/tlb")
        .arg(&tlb_path)
        .arg("/nologo")
        .status()
        .expect("failed to run midl (ensure Windows SDK is installed)");

    if !status.success() {
        panic!("midl failed");
    }

    tlb_path
}

fn write_rc(tlb_path: &Path, out_dir: &Path) -> PathBuf {
    let rc_path = out_dir.join("IntelHexExpand.rc");
    let rc_contents = format!("1 TYPELIB \"{}\"\n", tlb_path.to_string_lossy().replace('\\', "\\\\"));
    fs::write(&rc_path, rc_contents).expect("failed to write rc file");
    rc_path
}

fn main() {
    ensure_x64_target();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let idl_path = manifest_dir.join("IntelHexExpand.idl");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", idl_path.display());

    let tlb_path = run_midl(&idl_path, &out_dir);
    let rc_path = write_rc(&tlb_path, &out_dir);

    embed_resource::compile(rc_path, embed_resource::NONE);
}
