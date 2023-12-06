//! CI script generation
//!
//! In the future this may get split up into submodules.

use axoasset::LocalAsset;
use cargo_dist_schema::{GithubMatrix, GithubMatrixEntry};
use serde::Serialize;

use crate::{
    backend::{diff_files, templates::TEMPLATE_INSTALLER_DOCKER},
    config::{DependencyKind, SystemDependencies},
    errors::DistResult,
    DistGraph, SortedMap, SortedSet, TargetTriple,
};

/// Info about running cargo-dist in Github CI
#[derive(Debug, Serialize)]
pub struct DockerInfo {
    pub bin_names: Vec<String>,
    pub bin_paths: Vec<String>,
    pub builder: BuilderImage,
    pub runner: RunnerImage,
}

#[derive(Debug, Serialize)]
pub struct BuilderImage {
    pub image: String,
    pub apt_deps: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerImage {
    pub image: String,
    pub apt_deps: String,
}

impl DockerInfo {
    /// Compute the build stuff
    pub fn new(dist: &DistGraph) -> DockerInfo {
        for release in dist.releases {

        }

        DockerInfo {
            bin_names,
            bin_paths,
            builder,
            runner,
        }
    }

    fn dockerfile_path(&self, dist: &DistGraph) -> camino::Utf8PathBuf {
        let dir = dist.workspace_dir;
        dir.join("Dockerfile")
    }

    /// Generate the requested configuration and returns it as a string.
    pub fn generate_dockerfile(&self, dist: &DistGraph) -> DistResult<String> {
        let rendered = dist
            .templates
            .render_file_to_clean_string(TEMPLATE_INSTALLER_DOCKER, self)?;

        Ok(rendered)
    }

    /// Write release.yml to disk
    pub fn write_to_disk(&self, dist: &DistGraph) -> Result<(), miette::Report> {
        let ci_file = self.dockerfile_path(dist);
        let rendered = self.generate_github_ci(dist)?;

        LocalAsset::write_new_all(&rendered, &ci_file)?;
        eprintln!("generated Github CI to {}", ci_file);

        Ok(())
    }

    /// Check whether the new configuration differs from the config on disk
    /// writhout actually writing the result.
    pub fn check(&self, dist: &DistGraph) -> DistResult<()> {
        let ci_file = self.dockerfile_path(dist);

        let rendered = self.generate_dockerfile(dist)?;
        diff_files(&ci_file, &rendered)
    }
}

fn package_install_for_targets(
    targets: &Vec<&TargetTriple>,
    packages: &SystemDependencies,
) -> Option<String> {
    // TODO handle mixed-OS targets
    for target in targets {
        match target.as_str() {
            "i686-apple-darwin" | "x86_64-apple-darwin" | "aarch64-apple-darwin" => {
                let packages: Vec<String> = packages
                    .homebrew
                    .clone()
                    .into_iter()
                    .filter(|(_, package)| package.0.wanted_for_target(target))
                    .filter(|(_, package)| package.0.stage_wanted(&DependencyKind::Build))
                    .map(|(name, _)| name)
                    .collect();

                if packages.is_empty() {
                    return None;
                }

                return Some(brew_bundle_command(&packages));
            }
            "i686-unknown-linux-gnu"
            | "x86_64-unknown-linux-gnu"
            | "aarch64-unknown-linux-gnu"
            | "i686-unknown-linux-musl"
            | "x86_64-unknown-linux-musl"
            | "aarch64-unknown-linux-musl" => {
                let mut packages: Vec<String> = packages
                    .apt
                    .clone()
                    .into_iter()
                    .filter(|(_, package)| package.0.wanted_for_target(target))
                    .filter(|(_, package)| package.0.stage_wanted(&DependencyKind::Build))
                    .map(|(name, spec)| {
                        if let Some(version) = spec.0.version {
                            format!("{name}={version}")
                        } else {
                            name
                        }
                    })
                    .collect();

                // musl builds may require musl-tools to build;
                // necessary for more complex software
                if target.ends_with("linux-musl") {
                    packages.push("musl-tools".to_owned());
                }

                if packages.is_empty() {
                    return None;
                }

                let apts = packages.join(" ");
                return Some(format!("sudo apt-get install {apts}").to_owned());
            }
            "i686-pc-windows-msvc" | "x86_64-pc-windows-msvc" | "aarch64-pc-windows-msvc" => {
                let commands: Vec<String> = packages
                    .chocolatey
                    .clone()
                    .into_iter()
                    .filter(|(_, package)| package.0.wanted_for_target(target))
                    .filter(|(_, package)| package.0.stage_wanted(&DependencyKind::Build))
                    .map(|(name, package)| {
                        if let Some(version) = package.0.version {
                            format!("choco install {name} --version={version}")
                        } else {
                            format!("choco install {name}")
                        }
                    })
                    .collect();

                if commands.is_empty() {
                    return None;
                }

                return Some(commands.join("\n"));
            }
            _ => {}
        }
    }

    None
}
