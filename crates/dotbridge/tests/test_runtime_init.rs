//! Tests for DotBridgeRuntime initialization — equivalent to edge-js 101_edge_func.js

mod common;

use dotbridge::{DotBridgeConfig, DotBridgeRuntime};

#[test]
fn runtime_initializes_with_default_config() {
    let runtime = DotBridgeRuntime::new();
    assert!(runtime.is_ok(), "runtime should initialize with defaults: {:?}", runtime.err());
}

#[test]
fn runtime_initializes_with_custom_config() {
    let runtime = DotBridgeRuntime::with_config(DotBridgeConfig {
        debug: false,
        ..Default::default()
    });
    assert!(runtime.is_ok());
}

#[test]
fn runtime_bootstrap_dir_exists() {
    let runtime = common::init_runtime();
    let dir = runtime.bootstrap_dir();
    assert!(dir.exists(), "bootstrap dir should exist: {}", dir.display());
    assert!(
        dir.join("DotBridgeBootstrap.dll").is_file(),
        "DotBridgeBootstrap.dll should exist in bootstrap dir"
    );
}

#[test]
fn runtime_config_reflects_settings() {
    let runtime = DotBridgeRuntime::with_config(DotBridgeConfig {
        debug: true,
        target_framework: Some("net8.0".into()),
        ..Default::default()
    })
    .unwrap();

    assert!(runtime.config().debug);
    assert_eq!(runtime.config().target_framework.as_deref(), Some("net8.0"));
}

#[test]
fn func_from_source_requires_valid_csharp() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source("this is not valid C#");
    assert!(result.is_err(), "invalid C# should fail compilation");
}

#[test]
fn func_from_assembly_invalid_path_fails() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_assembly(
        "nonexistent.dll",
        "Namespace.Type, Assembly",
        "Method",
    );
    assert!(result.is_err(), "nonexistent assembly should fail");
}

#[test]
fn func_from_assembly_invalid_type_fails() {
    let runtime = common::init_runtime();
    let bootstrap_dll = runtime
        .bootstrap_dir()
        .join("DotBridgeBootstrap.dll")
        .to_string_lossy()
        .into_owned();

    let result = runtime.func_from_assembly(
        &bootstrap_dll,
        "NonExistent.Type, DotBridgeBootstrap",
        "Invoke",
    );
    assert!(result.is_err(), "nonexistent type should fail");
}
