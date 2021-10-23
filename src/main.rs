use anyhow::{anyhow, Result};
use tree_sitter::{Parser, Query, QueryCursor};
use structopt::StructOpt;
use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;

#[derive(StructOpt)]
struct Opt {
    #[structopt(long, parse(from_os_str))]
    root: PathBuf,

    #[structopt(long, parse(from_os_str))]
    output: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut parser = Parser::new();
    let lang = tree_sitter_coremake::language();
    parser.set_language(lang)?;

    let mut output = File::create(opt.output)?;
    writeln!(output, "digraph Uses {{")?;
    writeln!(output, "  ratio=1.3;")?;

    let text = std::fs::read_to_string(opt.root)?;
    let text = text.as_bytes();

    let tree = parser
        .parse(text, None)
        .ok_or_else(|| anyhow!("Could not parse input"))?;

    let definition_query = Query::new(lang, "(definition (identifier) @identifier (block) @block)")?;
    let use_query = Query::new(lang, "(use_statement (identifier) @identifier)")?;

    let mut cursor = QueryCursor::new();

    for m in cursor.matches(&definition_query, tree.root_node(), text) {
        let from = m.captures[0].node.utf8_text(text)?;

        let mut cursor = QueryCursor::new();

        for m in cursor.matches(&use_query, m.captures[1].node, text) {
            let to = m.captures[0].node.utf8_text(text)?;

            writeln!(output, "  \"{}\" -> \"{}\";", from, to)?;
        }
    }

    writeln!(output, "}}")?;

    Ok(())
}
