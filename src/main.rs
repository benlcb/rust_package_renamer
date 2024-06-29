use cargo_metadata::{camino::Utf8Path, MetadataCommand, Package};
use glob::glob;
use regex::Regex;
use std::fs;
use std::io::Write;
use toml_edit::{value, Document};

fn main() {
    // Load the workspace metadata
    let metadata = MetadataCommand::new()
        .current_dir("/home/bbarzen/workspace/aya")
        .exec()
        .expect("Failed to load metadata");

    let old_names = [
        "aya",
        "aya-log",
        "aya",
        "aya-log-common",
        "aya-log-parser",
        "aya-obj",
        "aya-tool",
        "aya-ebpf-macros",
        "aya-log-ebpf-macros",
        "aya-ebpf",
        "aya-ebpf-bindings",
        "aya-log-ebpf",
    ];
    let suffix = "-nightly-bbarzen";

    for old_name in old_names {
        let new_name = format!("{}{}", old_name, suffix);

        // Find the package to rename
        let package = metadata
            .packages
            .iter()
            .find(|pkg| pkg.name == old_name)
            .expect("Package not found");

        // Rename the package
        rename_package(package, &new_name);

        // Update dependencies
        for pkg in &metadata.packages {
            let cargo_toml = pkg.manifest_path.as_path();
            if cargo_toml.to_string().contains("index.crates.io") {
                continue;
            }
            println!("handling folder {}", cargo_toml);
            update_dependencies(cargo_toml, &old_name, &new_name);
            update_source_files(cargo_toml, &old_name, &new_name);
        }
    }
}

fn rename_package(package: &Package, new_name: &str) {
    let cargo_toml_path = package.manifest_path.as_path();
    let cargo_toml = fs::read_to_string(cargo_toml_path).expect("Failed to read Cargo.toml");
    let mut doc = cargo_toml
        .parse::<Document>()
        .expect("Failed to parse Cargo.toml");

    doc["package"]["name"] = value(new_name);

    fs::write(cargo_toml_path, doc.to_string()).expect("Failed to write Cargo.toml");
    println!("Renamed package {} to {}", package.name, new_name);
}

fn update_dependencies(cargo_toml: &Utf8Path, old_name: &str, new_name: &str) {
    let cargo_toml_content = fs::read_to_string(cargo_toml).expect("Failed to read Cargo.toml");
    let mut doc = cargo_toml_content
        .parse::<Document>()
        .expect("Failed to parse Cargo.toml");

    if let Some(dependencies) = doc.get_mut("dependencies") {
        if dependencies.get(old_name).is_some() {
            dependencies[new_name] = dependencies[old_name].clone();
            dependencies.as_table_mut().unwrap().remove(old_name);
        }
    }

    if let Some(dependencies) = doc.get_mut("dev-dependencies") {
        if dependencies.get(old_name).is_some() {
            dependencies[new_name] = dependencies[old_name].clone();
            dependencies.as_table_mut().unwrap().remove(old_name);
        }
    }

    fs::write(cargo_toml, doc.to_string()).expect("Failed to write Cargo.toml");
    //println!("Updated dependency {} for package {} to {}", old_name, package.name, new_name);
}

fn update_source_files(cargo_toml: &Utf8Path, old_name: &str, new_name: &str) {
    let pattern = format!("{}/**/*.rs", cargo_toml.parent().unwrap());
    //println!("Searching pattern {}", pattern);
    for sourcefile in glob(&pattern).expect("Failed to read glob pattern") {
        match sourcefile {
            Ok(sourcefile) => {
                let content = fs::read_to_string(sourcefile.clone()).unwrap();
                let content = update_use_statements(&content, old_name, new_name);
                let mut file = fs::File::create(sourcefile).unwrap();
                file.write_all(content.as_bytes()).unwrap();
                file.flush().unwrap();
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

fn update_use_statements(source: &str, old_name: &str, new_name: &str) -> String {
    let new_name = dash_to_underscore(new_name);
    let old_name = dash_to_underscore(old_name);
    let pattern = format!("use {}::", old_name);
    let replacement = format!("use {}::", new_name);
    let re = Regex::new(&pattern).unwrap();

    // Replace the pattern with the replacement string
    let haystack = re.replace_all(source, replacement);

    let pattern = format!("use {} as", old_name);
    let replacement = format!("use {} as", new_name);
    let re = Regex::new(&pattern).unwrap();

    re.replace_all(&haystack, replacement).to_string()
}

fn dash_to_underscore(name: &str) -> String {
    let re = Regex::new(r"-").unwrap();
    re.replace_all(name, "_").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_replace() {
        let test_content =
            "//Test comment use bla bla\nuse aya::{File};\nuse xyc::foo;\n".to_string();
        let result = update_use_statements(&test_content, "aya", "aya-example-suffix");
        let expected =
            "//Test comment use bla bla\nuse aya-example-suffix::{File};\nuse xyc::foo;\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_dash_to_underscore() {
        let result = dash_to_underscore("aya-example-prefix");
        assert_eq!(result, "aya_example_prefix");
    }
}
