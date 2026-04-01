use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=../src-sidecar");

  if env::var("PROFILE").as_deref() == Ok("release") {
    bundle_sidecar();
  } else {
    ensure_dev_sidecar_resource_dir();
  }

  tauri_build::build()
}

fn ensure_dev_sidecar_resource_dir() {
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
  let project_root = manifest_dir
    .parent()
    .expect("missing project root");
  let sidecar_resource_dir = project_root.join("dist/burgonet-sidecar");
  let sidecar_binary_path = project_root.join("src-tauri/binaries").join(sidecar_output_name());

  fs::create_dir_all(&sidecar_resource_dir).unwrap_or_else(|error| {
    panic!(
      "failed to create development sidecar resource directory at {}: {error}",
      sidecar_resource_dir.display()
    )
  });

  if let Some(parent) = sidecar_binary_path.parent() {
    fs::create_dir_all(parent).unwrap_or_else(|error| {
      panic!(
        "failed to create development sidecar binaries directory at {}: {error}",
        parent.display()
      )
    });
  }

  if !sidecar_binary_path.exists() {
    fs::write(&sidecar_binary_path, []).unwrap_or_else(|error| {
      panic!(
        "failed to create development sidecar placeholder at {}: {error}",
        sidecar_binary_path.display()
      )
    });
  }
}

fn bundle_sidecar() {
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
  let project_root = manifest_dir
    .parent()
    .expect("missing project root")
    .to_path_buf();
  let pyinstaller = pyinstaller_path(&project_root);
  let data_separator = if cfg!(target_os = "windows") { ";" } else { ":" };
  let add_data = format!("src-sidecar/models{data_separator}models");
  let output_dir = project_root.join("src-tauri/binaries");
  let sidecar_name = sidecar_output_name();

  if !pyinstaller.exists() {
    panic!(
      "PyInstaller was not found at {}. Install Python dependencies in .venv before building.",
      pyinstaller.display()
    );
  }

  fs::create_dir_all(&output_dir).unwrap_or_else(|error| {
    panic!("failed to create sidecar output directory at {}: {error}", output_dir.display())
  });

  let status = Command::new(&pyinstaller)
    .current_dir(&project_root)
    .args([
      "--noconfirm",
      "--clean",
      "--onefile",
      "--name",
      &sidecar_name,
      "--paths",
      "src-sidecar",
      "--add-data",
      &add_data,
      "--distpath",
      output_dir
        .to_str()
        .expect("sidecar output directory must be valid UTF-8"),
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

fn pyinstaller_path(project_root: &std::path::Path) -> PathBuf {
  if cfg!(target_os = "windows") {
    project_root.join(".venv/Scripts/pyinstaller.exe")
  } else {
    project_root.join(".venv/bin/pyinstaller")
  }
}

fn sidecar_output_name() -> String {
  let target = env::var("TARGET").expect("missing target triple");
  if cfg!(target_os = "windows") {
    format!("burgonet-sidecar-{target}.exe")
  } else {
    format!("burgonet-sidecar-{target}")
  }
}
