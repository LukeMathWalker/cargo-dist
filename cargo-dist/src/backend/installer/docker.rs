//! CI script generation
//!
//! In the future this may get split up into submodules.

use std::process::Command;

use axoasset::LocalAsset;
use camino::Utf8PathBuf;
use serde::Serialize;

use crate::{
    backend::templates::{Templates, TEMPLATE_INSTALLER_DOCKER},
    errors::DistResult,
};

/// Info needed to build an msi
#[derive(Debug, Clone)]
pub struct DockerInstallerInfo {
    /// Binaries we'll be baking into the docker image
    pub bins: Vec<String>,
    /// Final file path of the docker image
    pub file_path: Utf8PathBuf,
    /// Dir stuff goes to
    pub package_dir: Utf8PathBuf,
}

/// Info about running cargo-dist in Github CI
#[derive(Debug, Serialize)]
pub struct DockerfileInfo {
    bins: Vec<Bin>,
    runner: RunnerImage,
}

#[derive(Debug, Serialize)]
struct Bin {
    source: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct RunnerImage {
    image: String,
    apt_deps: Option<String>,
}

const RUNNER_IMAGE: &str = "bookworm-slim";
const DOCKERFILE_NAME: &str = "Dockerfile";

impl DockerInstallerInfo {
    /// Build the msi installer
    ///
    /// Note that this assumes `write_wsx_to_disk` was run beforehand (via `cargo dist generate`),
    /// which should be enforced by `check_wsx` (via `cargo dist generate --check`).
    pub fn build(&self, templates: &Templates) -> DistResult<()> {
        eprintln!("time to DOCK");

        let info = self.dockerfile();
        let contents = templates.render_file_to_clean_string(TEMPLATE_INSTALLER_DOCKER, &info)?;
        let dockerfile_path = self.package_dir.join(DOCKERFILE_NAME);
        LocalAsset::write_new(&contents, dockerfile_path)?;

        let mut cmd = Command::new("docker");
        cmd.arg("build");
        cmd.status().unwrap();

        eprintln!("yoooo");
        Ok(())
    }

    fn dockerfile(&self) -> DockerfileInfo {
        let bins = self
            .bins
            .iter()
            .map(|file_name| Bin {
                source: format!("./{file_name}"),
                name: file_name.clone(),
            })
            .collect();
        let runner = RunnerImage {
            image: RUNNER_IMAGE.to_owned(),
            apt_deps: None,
        };
        DockerfileInfo { bins, runner }
    }
}

/*
impl DockerInfo {
    /// Compute the build stuff
    pub fn new(dist: &DistGraph) -> DockerInfo {
        for release in &dist.releases {
            let mut bins = vec![];
            let mut apt_deps = None;
            for &variant_idx in &release.variants {
                let variant = dist.variant(variant_idx);
                if variant.target != TARGET_X64_LINUX_GNU {
                    continue;
                }
                for &artifact_idx in &variant.local_artifacts {
                    let artifact = dist.artifact(artifact_idx);
                    let ArtifactKind::ExecutableZip(_) = &artifact.kind else {
                        continue;
                    };
                    let Some(archive) = &artifact.archive else {
                        continue;
                    };
                    for &bin_idx in &variant.binaries {
                        let bin = dist.binary(bin_idx);
                        let path = archive.dir_path.join(&bin.file_name);
                        bins.push(Bin { source: path.to_string(), name: bin.name.clone() });
                    }
                }
            }
            if !bins.is_empty() {
                return DockerInfo {
                    bins,
                    runner: RunnerImage { image: RUNNER_IMAGE.to_owned(), apt_deps },
                }
            }
        }
        unreachable!()

    }

    fn dockerfile_path(&self, dist: &DistGraph) -> camino::Utf8PathBuf {
        let dir = &dist.workspace_dir;
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
        let rendered = self.generate_dockerfile(dist)?;

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
 */
/*

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

*/
