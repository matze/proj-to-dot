use anyhow::{anyhow, Result};
use log::debug;
use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tree_sitter::{Language, Parser, Query, QueryCursor};

#[derive(StructOpt)]
struct Opt {
    #[structopt(long, parse(from_os_str))]
    root: PathBuf,

    #[structopt(long, parse(from_os_str))]
    output: PathBuf,
}

struct App {
    lang: Language,
    output: RefCell<File>,
    include_query: Query,
    definition_query: Query,
    use_query: Query,
}

impl App {
    fn new(opt: &Opt) -> Result<Self> {
        let output = RefCell::new(File::create(&opt.output)?);
        let lang = tree_sitter_coremake::language();

        Ok(Self {
            lang,
            output,
            include_query: Query::new(lang, "(include (string_literal) @glob)")?,
            definition_query: Query::new(
                lang,
                "(definition (identifier) @identifier (block) @block)",
            )?,
            use_query: Query::new(lang, "(use_statement (identifier) @identifier)")?,
        })
    }

    fn parse_real(&self, proj: &Path) -> Result<()> {
        let mut parser = Parser::new();
        parser.set_language(self.lang)?;
        debug!("Parsing {:?}", proj);

        let text = std::fs::read_to_string(proj)?;
        let text = text.as_bytes();

        let tree = parser
            .parse(text, None)
            .ok_or_else(|| anyhow!("Could not parse input"))?;

        let mut cursor = QueryCursor::new();

        for include in cursor.matches(&self.include_query, tree.root_node(), text) {
            let pattern = include.captures[0].node.utf8_text(text)?.trim_matches('"');
            let base = proj.parent().ok_or_else(|| anyhow!("No parent"))?;
            let walker = globwalk::GlobWalkerBuilder::from_patterns(base, &[pattern])
                .build()?
                .into_iter()
                .filter_map(Result::ok);

            for proj in walker {
                self.parse_real(proj.path())?;
            }
        }

        let mut output = self.output.borrow_mut();

        for def in cursor.matches(&self.definition_query, tree.root_node(), text) {
            let from = def.captures[0].node.utf8_text(text)?;

            let mut cursor = QueryCursor::new();

            for use_target in cursor.matches(&self.use_query, def.captures[1].node, text) {
                let to = use_target.captures[0].node.utf8_text(text)?;
                writeln!(output, "  \"{}\" -> \"{}\";", from, to)?;
            }
        }
        Ok(())
    }

    fn parse(&self, proj: &Path) -> Result<()> {
        {
            let mut output = self.output.borrow_mut();

            writeln!(output, "digraph Uses {{")?;
            writeln!(output, "  ratio=1.3;")?;
        }

        self.parse_real(proj)?;

        let mut output = self.output.borrow_mut();
        writeln!(output, "}}")?;

        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Opt::from_args();
    App::new(&opt)?.parse(&opt.root)?;

    Ok(())
}
