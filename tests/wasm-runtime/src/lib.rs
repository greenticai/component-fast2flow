//! WASM Runtime tests for fast2flow components.
//!
//! These tests validate that the compiled WASM components can be loaded
//! and instantiated by the wasmtime runtime.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .expect("workspace root")
    }

    fn build_wasm_components() -> anyhow::Result<()> {
        let workspace = workspace_root();
        let status = Command::new("cargo")
            .current_dir(&workspace)
            .args([
                "build",
                "--target",
                "wasm32-wasip2",
                "--release",
                "-p",
                "indexer",
                "-p",
                "matcher",
                "-p",
                "router",
            ])
            .status()?;

        if !status.success() {
            anyhow::bail!("WASM build failed");
        }
        Ok(())
    }

    fn wasm_path(component: &str) -> PathBuf {
        workspace_root()
            .join("target/wasm32-wasip2/release")
            .join(format!("{}.wasm", component))
    }

    /// Test that indexer WASM component can be loaded
    #[test]
    fn test_indexer_wasm_loads() -> anyhow::Result<()> {
        build_wasm_components()?;

        let wasm_path = wasm_path("indexer");
        assert!(
            wasm_path.exists(),
            "indexer.wasm should exist at {:?}",
            wasm_path
        );

        // Create wasmtime engine and try to compile the component
        let engine = wasmtime::Engine::default();
        let wasm_bytes = std::fs::read(&wasm_path)?;

        // Try to compile as a component
        let component = wasmtime::component::Component::new(&engine, &wasm_bytes)?;

        // Verify component was created successfully
        assert!(
            component.component_type().exports(&engine).count() > 0,
            "Component should have exports"
        );

        println!(
            "indexer.wasm loaded successfully, size: {} bytes",
            wasm_bytes.len()
        );
        Ok(())
    }

    /// Test that matcher WASM component can be loaded
    #[test]
    fn test_matcher_wasm_loads() -> anyhow::Result<()> {
        build_wasm_components()?;

        let wasm_path = wasm_path("matcher");
        assert!(
            wasm_path.exists(),
            "matcher.wasm should exist at {:?}",
            wasm_path
        );

        // Create wasmtime engine and try to compile the component
        let engine = wasmtime::Engine::default();
        let wasm_bytes = std::fs::read(&wasm_path)?;

        // Try to compile as a component
        let component = wasmtime::component::Component::new(&engine, &wasm_bytes)?;

        // Verify component was created successfully
        assert!(
            component.component_type().exports(&engine).count() > 0,
            "Component should have exports"
        );

        println!(
            "matcher.wasm loaded successfully, size: {} bytes",
            wasm_bytes.len()
        );
        Ok(())
    }

    /// Test that router WASM component can be loaded
    #[test]
    fn test_router_wasm_loads() -> anyhow::Result<()> {
        build_wasm_components()?;

        let wasm_path = wasm_path("router");
        assert!(
            wasm_path.exists(),
            "router.wasm should exist at {:?}",
            wasm_path
        );

        // Create wasmtime engine and try to compile the component
        let engine = wasmtime::Engine::default();
        let wasm_bytes = std::fs::read(&wasm_path)?;

        // Try to compile as a component
        let component = wasmtime::component::Component::new(&engine, &wasm_bytes)?;

        // Verify component was created successfully
        assert!(
            component.component_type().exports(&engine).count() > 0,
            "Component should have exports"
        );

        println!(
            "router.wasm loaded successfully, size: {} bytes",
            wasm_bytes.len()
        );
        Ok(())
    }

    /// Test WASM component file sizes are reasonable
    #[test]
    fn test_wasm_sizes_reasonable() -> anyhow::Result<()> {
        build_wasm_components()?;

        for component in ["indexer", "matcher", "router"] {
            let path = wasm_path(component);
            let metadata = std::fs::metadata(&path)?;
            let size_kb = metadata.len() / 1024;

            // Components should be less than 10MB (reasonable size)
            assert!(
                size_kb < 10 * 1024,
                "{}.wasm is too large: {} KB",
                component,
                size_kb
            );

            // Components should be at least 1KB (not empty)
            assert!(
                size_kb >= 1,
                "{}.wasm is too small: {} KB",
                component,
                size_kb
            );

            println!("{}.wasm size: {} KB", component, size_kb);
        }
        Ok(())
    }
}
