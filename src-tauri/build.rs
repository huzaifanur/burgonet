use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=../src-sidecar");

  if env::var("PROFILE").as_deref() == Ok("release") {
    bundle_sidecar();
  }

  tauri_build::build()
}

fn bundle_sidecar() {
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
  let project_root = manifest_dir
    .parent()
    .expect("missing project root")
    .to_path_buf();
  let pyinstaller = project_root.join(".venv/bin/pyinstaller");

  if !pyinstaller.exists() {
    panic!(
      "PyInstaller was not found at {}. Install Python dependencies in .venv before building.",
      pyinstaller.display()
    );
  }

  let status = Command::new(&pyinstaller)
    .current_dir(&project_root)
    .args([
      "--noconfirm",
      "--clean",
      "--onedir",
      "--name",
      "burgonet-sidecar",
      "--paths",
      "src-sidecar",
      "--add-data",
      "src-sidecar/models:models",
      "--collect-all",
      "mediapipe",
      "src-sidecar/main.py",
    ])
    .status()
    .expect("failed to launch PyInstaller");

  if !status.success() {
    panic!("PyInstaller sidecar build failed with status {status}");
  }
}
