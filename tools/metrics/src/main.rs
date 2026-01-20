use std::collections::HashMap;
use std::fs;
use std::path::Path;
use syn::visit::Visit;
use syn::{File, ImplItem, ItemFn, ItemImpl};
use walkdir::WalkDir;

#[derive(Default)]
struct Metrics {
    impl_methods: HashMap<String, Vec<MethodInfo>>,
    functions: Vec<FunctionInfo>,
}

struct MethodInfo {
    name: String,
    file: String,
    line: usize,
    lines: usize,
}

struct FunctionInfo {
    name: String,
    file: String,
    line: usize,
    lines: usize,
    params: usize,
}

struct MetricsVisitor<'a> {
    file_path: &'a str,
    metrics: &'a mut Metrics,
    source: &'a str,
}

impl<'ast> Visit<'ast> for MetricsVisitor<'_> {
    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let type_name = if let syn::Type::Path(p) = &*node.self_ty {
            p.path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_else(|| "Unknown".into())
        } else {
            "Unknown".into()
        };

        let key = format!("{}::{}", self.file_path, type_name);

        for item in &node.items {
            if let ImplItem::Fn(method) = item {
                let start = method.sig.ident.span().start();
                let end = method.block.brace_token.span.close().end();
                let lines = end.line.saturating_sub(start.line) + 1;

                self.metrics.impl_methods.entry(key.clone()).or_default().push(
                    MethodInfo {
                        name: method.sig.ident.to_string(),
                        file: self.file_path.to_string(),
                        line: start.line,
                        lines,
                    },
                );
            }
        }

        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let start = node.sig.ident.span().start();
        let end = node.block.brace_token.span.close().end();
        let lines = end.line.saturating_sub(start.line) + 1;
        let params = node.sig.inputs.len();

        self.metrics.functions.push(FunctionInfo {
            name: node.sig.ident.to_string(),
            file: self.file_path.to_string(),
            line: start.line,
            lines,
            params,
        });

        syn::visit::visit_item_fn(self, node);
    }
}

fn analyze_file(path: &Path, metrics: &mut Metrics) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let syntax: File = syn::parse_file(&content)?;

    let file_path = path.to_string_lossy();
    let mut visitor = MetricsVisitor {
        file_path: &file_path,
        metrics,
        source: &content,
    };

    visitor.visit_file(&syntax);
    Ok(())
}

fn main() {
    let src_dir = std::env::args().nth(1).unwrap_or_else(|| "src".into());

    let mut metrics = Metrics::default();

    for entry in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        if let Err(e) = analyze_file(entry.path(), &mut metrics) {
            eprintln!("Failed to parse {}: {}", entry.path().display(), e);
        }
    }

    // Report: Types with many methods
    println!("## Types by method count\n");
    let mut impl_counts: Vec<_> = metrics
        .impl_methods
        .iter()
        .map(|(k, v)| (k, v.len()))
        .collect();
    impl_counts.sort_by(|a, b| b.1.cmp(&a.1));

    println!("| Type | Methods |");
    println!("|------|---------|");
    for (name, count) in impl_counts.iter().take(15) {
        let short_name = name.rsplit("src/").next().unwrap_or(name);
        println!("| {} | {} |", short_name, count);
    }

    // Warn about types with too many methods
    let threshold = 20;
    let large_types: Vec<_> = impl_counts.iter().filter(|(_, c)| *c > threshold).collect();
    if !large_types.is_empty() {
        println!("\n**Warning:** {} types have >{} methods\n", large_types.len(), threshold);
    }

    // Report: Long functions
    println!("\n## Longest functions\n");
    let mut fns = metrics.functions.clone();
    fns.sort_by(|a, b| b.lines.cmp(&a.lines));

    println!("| Function | File | Lines |");
    println!("|----------|------|-------|");
    for f in fns.iter().take(10) {
        let short_file = f.file.rsplit("src/").next().unwrap_or(&f.file);
        println!("| {} | {}:{} | {} |", f.name, short_file, f.line, f.lines);
    }

    // Report: Functions with many parameters
    println!("\n## Functions by parameter count\n");
    let mut by_params: Vec<_> = metrics.functions.iter().filter(|f| f.params > 4).collect();
    by_params.sort_by(|a, b| b.params.cmp(&a.params));

    if by_params.is_empty() {
        println!("No functions with >4 parameters.");
    } else {
        println!("| Function | File | Params |");
        println!("|----------|------|--------|");
        for f in by_params.iter().take(10) {
            let short_file = f.file.rsplit("src/").next().unwrap_or(&f.file);
            println!("| {} | {}:{} | {} |", f.name, short_file, f.line, f.params);
        }
    }

    // Summary
    let total_methods: usize = metrics.impl_methods.values().map(|v| v.len()).sum();
    let total_fns = metrics.functions.len();
    println!("\n---");
    println!("Total impl methods: {}", total_methods);
    println!("Total standalone functions: {}", total_fns);
}
