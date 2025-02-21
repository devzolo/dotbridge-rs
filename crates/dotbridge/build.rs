use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let dotnet_dir = Path::new("dotnet/DotBridgeBootstrap");
    let csproj = dotnet_dir.join("DotBridgeBootstrap.csproj");

    if !csproj.exists() {
        return;
    }

    println!("cargo:rerun-if-changed=dotnet/DotBridgeBootstrap/Compiler.cs");
    println!("cargo:rerun-if-changed=dotnet/DotBridgeBootstrap/Invoker.cs");
    println!("cargo:rerun-if-changed=dotnet/DotBridgeBootstrap/WireProtocol.cs");
    println!("cargo:rerun-if-changed=dotnet/DotBridgeBootstrap/DotBridgeBootstrap.csproj");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let publish_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .join("dotbridge-bootstrap");

    let dotnet_cmd = find_dotnet();

    let status = Command::new(&dotnet_cmd)
        .arg("publish")
        .arg(csproj.to_str().unwrap())
        .arg("-c")
        .arg("Release")
        .arg("-o")
        .arg(publish_dir.to_str().unwrap())
        .arg("--nologo")
        .arg("-v")
        .arg("quiet")
        .status();

    match status {
        Ok(s) if s.success() => {
            let config = r#"{
  "runtimeOptions": {
    "tfm": "net8.0",
    "rollForward": "Major",
    "framework": {
      "name": "Microsoft.NETCore.App",
      "version": "8.0.0"
    }
  }
}"#;
            let config_path = publish_dir.join("DotBridgeBootstrap.runtimeconfig.json");
            std::fs::write(&config_path, config).ok();

            println!(
                "cargo:warning=DotBridgeBootstrap built to: {}",
                publish_dir.display()
            );
        }
        Ok(s) => {
            println!(
                "cargo:warning=dotnet publish failed (exit {s}). \
                 C# compilation will not be available at runtime."
            );
        }
        Err(e) => {
            println!(
                "cargo:warning=dotnet not found ({e}). \
                 Install .NET SDK 8.0+ to enable C# compilation."
            );
        }
    }
}

fn find_dotnet() -> PathBuf {
    let candidates = [
        PathBuf::from("dotnet"),
        PathBuf::from(r"C:\Program Files\dotnet\dotnet.exe"),
        PathBuf::from(r"C:\Program Files (x86)\dotnet\dotnet.exe"),
    ];

    for candidate in &candidates {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return candidate.clone();
        }
    }

    PathBuf::from("dotnet")
}
