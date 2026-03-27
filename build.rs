use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    for path in [
        "build.rs",
        "ui/package.json",
        "ui/pnpm-lock.yaml",
        "ui/index.html",
        "ui/vite.config.ts",
        "ui/eslint.config.js",
        "ui/tsconfig.json",
        "ui/tsconfig.app.json",
        "ui/tsconfig.node.json",
        "ui/src",
        "ui/public",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }

    let ui_dir = Path::new("ui");
    if !ui_dir.exists() {
        panic!("missing ui directory at {}", ui_dir.display());
    }

    run_pnpm(ui_dir, &["install"]);
    run_pnpm(ui_dir, &["build"]);
}

fn run_pnpm(ui_dir: &Path, args: &[&str]) {
    println!("cargo:warning=running pnpm {}", args.join(" "));

    let status = Command::new(pnpm_command())
        .args(args)
        .current_dir(ui_dir)
        .status()
        .unwrap_or_else(|error| {
            panic!(
                "failed to start pnpm in {}: {error}. Install pnpm and ensure it is on PATH.",
                ui_dir.display()
            )
        });

    if !status.success() {
        panic!(
            "pnpm {} failed in {} with status {status}",
            args.join(" "),
            ui_dir.display()
        );
    }
}

fn pnpm_command() -> &'static str {
    if env::consts::OS == "windows" {
        "pnpm.cmd"
    } else {
        "pnpm"
    }
}
