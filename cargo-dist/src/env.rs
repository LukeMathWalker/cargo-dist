//! Functions to parse and manipulate the environment

use std::{env, process::Command};

use camino::Utf8Path;
use miette::{Context, IntoDiagnostic};

use crate::{
    errors::{DistError, DistResult, Result},
    DistGraph, SortedMap,
};

/// Fetches the Homebrew environment from `brew bundle exec`
pub fn fetch_brew_env(dist_graph: &DistGraph) -> Result<Option<String>> {
    if let Some(brew) = &dist_graph.tools.brew {
        if Utf8Path::new("Brewfile").exists() {
            // Uses `brew bundle exec` to just print its own environment,
            // allowing us to capture what it generated and decide what
            // to do with it.
            let result = Command::new(&brew.cmd)
                .arg("bundle")
                .arg("exec")
                .arg("--")
                .arg("/usr/bin/env")
                .output()
                .into_diagnostic()
                .wrap_err_with(|| "failed to exec brew bundle exec".to_string())?;

            return Ok(Some(String::from_utf8_lossy(&result.stdout).to_string()));
        }
    }

    Ok(None)
}

/// Takes a string in KEY=value environment variable format and
/// parses it into a BTreeMap. The string syntax is sh-compatible, and also the
/// format returned by `env`.
/// Note that we trust the parsed string to contain a given key only once;
/// if specified more than once, only the final occurrence will be included.
pub fn parse_env(env_string: &str) -> DistResult<SortedMap<&str, &str>> {
    let mut parsed = SortedMap::new();
    for line in env_string.trim_end().split('\n') {
        let Some((key, value)) = line.split_once('=') else {
            return Err(DistError::EnvParseError {
                line: line.to_owned(),
            });
        };
        parsed.insert(key, value);
    }

    Ok(parsed)
}

/// Given the environment captured from `brew bundle exec -- env`, returns
/// a list of all dependencies from that environment and the opt prefixes
/// to those packages.
fn formulas_from_env(environment: &SortedMap<&str, &str>) -> Vec<(String, String)> {
    let mut packages = vec![];

    // Set by Homebrew/brew bundle - a comma-separated list of all
    // dependencies in the recursive tree calculated from the dependencies
    // in the Brewfile.
    if let Some(formulastring) = environment.get("HOMEBREW_DEPENDENCIES") {
        // Set by Homebrew/brew bundle - the path to Homebrew's "opt"
        // directory, which is where links to the private cellar of every
        // installed package lives.
        // Usually /opt/homebrew/opt or /usr/local/opt.
        if let Some(opt_prefix) = environment.get("HOMEBREW_OPT") {
            for dep in formulastring.split(',') {
                // Unwrap here is safe because `split` will always return
                // a collection of at least one item.
                let short_name = dep.split('/').last().unwrap();
                let pkg_opt = format!("{opt_prefix}/{short_name}");
                packages.push((dep.to_owned(), pkg_opt));
            }
        }
    }

    packages
}

/// Takes a BTreeMap of key/value environment variables produced by
/// `brew bundle exec` and decides which ones we want to keep for our own builds.
/// Returns a Vec containing (KEY, value) tuples.
pub fn select_brew_env(environment: &SortedMap<&str, &str>) -> Vec<(String, String)> {
    let mut desired_env = vec![];

    // Several of Homebrew's environment variables are safe for us to use
    // unconditionally, so pick those in their entirety.
    if let Some(value) = environment.get("PKG_CONFIG_PATH") {
        desired_env.push(("PKG_CONFIG_PATH".to_owned(), value.to_string()))
    }
    if let Some(value) = environment.get("PKG_CONFIG_LIBDIR") {
        desired_env.push(("PKG_CONFIG_LIBDIR".to_owned(), value.to_string()))
    }
    if let Some(value) = environment.get("CMAKE_INCLUDE_PATH") {
        desired_env.push(("CMAKE_INCLUDE_PATH".to_owned(), value.to_string()))
    }
    if let Some(value) = environment.get("CMAKE_LIBRARY_PATH") {
        desired_env.push(("CMAKE_LIBRARY_PATH".to_owned(), value.to_string()))
    }
    let mut paths = vec![];

    // For each listed dependency, add it to the PATH
    for (_, pkg_opt) in formulas_from_env(environment) {
        // Not every package will have a /bin or /sbin directory,
        // but it's safe to add both to the PATH just in case.
        paths.push(format!("{pkg_opt}/bin"));
        paths.push(format!("{pkg_opt}/sbin"));
    }

    if !paths.is_empty() {
        if let Ok(our_path) = env::var("PATH") {
            let desired_path = format!("{our_path}:{}", paths.join(":"));

            desired_env.insert(0, ("PATH".to_owned(), desired_path));
        }
    }

    desired_env
}

/// Determines the flags needed by the linker to link against
/// Homebrew packages in the provided environment.
/// Note that this may reference directories which don't exist;
/// this function doesn't validate the existence of directories in the
/// generated flags.
pub fn calculate_ldflags(environment: &SortedMap<&str, &str>) -> String {
    formulas_from_env(environment)
        .iter()
        .map(|(_, pkg_opt)| format!("-L{pkg_opt}/lib"))
        .collect::<Vec<String>>()
        .join(" ")
}

/// Determines the flags needed by the compiler to locate headers
/// from Homebrew packages in the provided environment.
/// Note that this may reference directories which don't exist;
/// this function doesn't validate the existence of directories in the
/// generated flags.
pub fn calculate_cflags(environment: &SortedMap<&str, &str>) -> String {
    formulas_from_env(environment)
        .iter()
        .map(|(_, pkg_opt)| format!("-I{pkg_opt}/include"))
        .collect::<Vec<String>>()
        .join(" ")
}
