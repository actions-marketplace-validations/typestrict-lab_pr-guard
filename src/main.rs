use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use pr_guard::format::{self, Format};
use pr_guard::policy::{Mode, Policy};
use pr_guard::{ci, diff, git, model, report, rules, verdict};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ModeArg {
    Shadow,
    Enforce,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // parsed once
enum Cmd {
    /// Decide a pull request: the policy over the diff and TS-PRGuard's answers.
    Check {
        /// A unified diff to read instead of git (`-` for stdin).
        #[arg(long, value_name = "FILE", conflicts_with_all = ["staged", "base", "head"])]
        diff: Option<PathBuf>,
        /// Compare the staged changes with HEAD (pre-commit) instead of a commit range.
        #[arg(long, conflicts_with_all = ["base", "head"])]
        staged: bool,
        /// The base of the range; default the PR's target branch in CI, else origin/HEAD, main or master. The range starts
        /// at its merge base with the head.
        #[arg(long, value_name = "REF")]
        base: Option<String>,
        /// The head of the range; default HEAD.
        #[arg(long, value_name = "REF")]
        head: Option<String>,
        /// TS-PRGuard's answers as JSON, instead of running the model.
        #[arg(long)]
        answers: Option<PathBuf>,
        /// The pull request's description (title and body); default the PR's in CI, else the range's commit messages.
        #[arg(long, conflicts_with = "description_file")]
        description: Option<String>,
        #[arg(long)]
        description_file: Option<PathBuf>,
        /// Only the deterministic rules: no model, instant (the pre-commit default).
        #[arg(long, conflicts_with = "answers")]
        no_model: bool,
        /// Directory with the model weights; default `$PR_GUARD_MODEL`, else the cache (`$PR_GUARD_CACHE`, `~/.cache/pr-guard`).
        #[arg(long)]
        model: Option<PathBuf>,
        /// ONNX Runtime threads; default all cores.
        #[arg(long)]
        threads: Option<usize>,
        /// Missing file = the default policy.
        #[arg(long, default_value = ".github/pr-guard.yml")]
        policy: PathBuf,
        /// Overrides the policy's mode.
        #[arg(long, value_enum)]
        mode: Option<ModeArg>,
        /// The pull request's labels (repeat the flag).
        #[arg(long = "label")]
        labels: Vec<String>,
        /// Output format; repeat it with one --output each. Default: text to stdout.
        #[arg(long, value_enum)]
        format: Vec<Format>,
        /// Where each --format goes, in the same order (`-` for stdout); one --format alone goes to stdout.
        #[arg(long, value_name = "PATH")]
        output: Vec<PathBuf>,
        /// Where to write the sticky comment's markdown (same as `--format markdown --output <PATH>`).
        #[arg(long, value_name = "PATH")]
        comment: Option<PathBuf>,
    },
    /// Print the default policy, to start a `.github/pr-guard.yml`.
    Init,
}

fn main() {
    // 0 passed, 1 blocked by the policy, 2 usage or runtime error.
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(2);
    }
}

fn run() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Init => print!("{}", serde_yaml::to_string(&Policy::default())?),
        Cmd::Check {
            diff: diff_path, staged, base, head, answers, description, description_file, no_model, model: model_dir, threads, policy,
            mode, labels, format, output, comment,
        } => {
            let targets = targets(format, output, comment)?;
            let mut policy = Policy::load(&policy)?;
            if let Some(m) = mode {
                policy.mode = match m {
                    ModeArg::Shadow => Mode::Shadow,
                    ModeArg::Enforce => Mode::Enforce,
                };
            }
            let ci = ci::detect(&ci::Env::from_process());
            let cwd = std::path::Path::new(".");
            let source = match &diff_path {
                Some(_) => None,
                None if staged => Some(git::Source::Staged),
                None => Some(git::range(cwd, base.as_deref(), head.as_deref(), ci.as_ref())?),
            };
            let raw = match (&diff_path, &source) {
                (Some(p), _) if p.as_os_str() == "-" => {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf).context("reading the diff from stdin")?;
                    String::from_utf8_lossy(&buf).into_owned()
                }
                // As the lab mined it (`errors="replace"`): invalid UTF-8 becomes U+FFFD instead of an error.
                (Some(p), _) => String::from_utf8_lossy(&std::fs::read(p).with_context(|| format!("reading {}", p.display()))?).into_owned(),
                (None, Some(s)) => git::diff(cwd, s)?,
                (None, None) => unreachable!("a source when there is no --diff"),
            };
            let files = diff::parse(&raw);
            // The PR's own description first; commit messages only when there is no PR at hand.
            let description = match description_file {
                Some(p) => Some(std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?),
                None => match description.or_else(|| ci.as_ref().and_then(ci::Ci::description)) {
                    Some(d) => Some(d),
                    None => match &source {
                        Some(s) => git::messages(cwd, s)?,
                        None => None,
                    },
                },
            };
            let mut answers = match answers {
                Some(p) => serde_json::from_str(&std::fs::read_to_string(&p)?).with_context(|| format!("reading {}", p.display()))?,
                None => verdict::Answers::default(),
            };
            if raw.trim().is_empty() {
                eprintln!("pr-guard: no changes to check");
            } else if !no_model && answers == verdict::Answers::default() {
                ask_model(&mut answers, model_dir, threads, description.as_deref(), &raw)?;
            }
            let found = rules::check(&policy.rules, &files)?;
            let v = verdict::decide(&policy, &files, &answers, &labels, found);
            // Workflow commands mean nothing outside GitHub Actions.
            if ci.as_ref().is_some_and(|c| c.provider == ci::Provider::GitHub) {
                print!("{}", report::annotations(&v));
            }
            let color = format::use_color(std::io::IsTerminal::is_terminal(&std::io::stdout()), std::env::var("NO_COLOR").ok().as_deref());
            for (f, path) in targets {
                match path {
                    Some(p) => std::fs::write(&p, format::render(f, &v, false)).with_context(|| format!("writing {}", p.display()))?,
                    None => print!("{}", format::render(f, &v, color)),
                }
            }
            if v.blocked {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

/// Pairs each --format with its --output (`None` = stdout): one --output per --format in order, or a single --format to
/// stdout; --comment adds the markdown.
fn targets(formats: Vec<Format>, outputs: Vec<PathBuf>, comment: Option<PathBuf>) -> anyhow::Result<Vec<(Format, Option<PathBuf>)>> {
    let stdout = |p: PathBuf| (p.as_os_str() != "-").then_some(p);
    let mut t = match (formats.len(), outputs.len()) {
        (0, 0) => vec![(Format::Text, None)],
        (0, _) => anyhow::bail!("--output needs a --format before it"),
        (1, 0) => vec![(formats[0], None)],
        (f, o) if f == o => formats.into_iter().zip(outputs.into_iter().map(stdout)).collect(),
        (f, o) => anyhow::bail!("{f} --format but {o} --output: give one --output per --format (`-` for stdout)"),
    };
    if let Some(p) = comment {
        t.push((Format::Markdown, Some(p)));
    }
    Ok(t)
}

fn ask_model(answers: &mut verdict::Answers, dir: Option<PathBuf>, threads: Option<usize>, description: Option<&str>, raw_diff: &str) -> anyhow::Result<()> {
    let description = description.map(model::py_strip).filter(|d| !d.is_empty());
    let dir = dir.or_else(model::default_dir).context("no model directory: pass --model, set PR_GUARD_MODEL or HOME")?;
    anyhow::ensure!(
        dir.join("decision_ai.json").exists() || dir.join("pr_guard_model.json").exists(),
        "no model weights in {} (pass --model <dir>, set PR_GUARD_MODEL, or run with --no-model; the download from Hugging Face is not implemented yet)",
        dir.display()
    );
    let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let t0 = std::time::Instant::now();
    let mut m = model::Model::load(&dir, threads)?;
    let loaded = t0.elapsed();
    let cut = model::cut_diff(raw_diff);
    let argmax = |p: &[f64]| p.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map_or(0, |(i, _)| i);
    let tasks: Vec<&model::Task> = model::TASKS.iter().filter(|t| m.knows(t.name)).collect();
    for task in tasks {
        let Some(p) = m.ask(task, description, &cut)? else {
            eprintln!("pr-guard: {} skipped (no description)", task.name);
            continue;
        };
        let best = argmax(&p);
        eprintln!("pr-guard: {:<18} {:>5.3}  {}", task.name, p[best], task.options[best]);
        match task.name {
            "pr-descricao-noul" => answers.description_matches = Some(p[1]),
            "prg-vul" => answers.vulnerability = Some(p[1]),
            "prg-quebra" => answers.breaking = Some(p[1]),
            // Not applicable (docs, configuration) is the safe side too.
            "prg-testes" => answers.tests_cover = Some(1.0 - p[1]),
            "prg-tipo" => answers.change_type = Some(task.options[best].to_owned()),
            "prg-complexidade" => answers.complexity = Some(best as u8),
            // prg-quebra-api: weak on the held-out split (F1 0.48), reported but not used until it is retrained.
            _ => {}
        }
    }
    eprintln!("pr-guard: model loaded in {:.1}s, answered in {:.1}s", loaded.as_secs_f64(), (t0.elapsed() - loaded).as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_pair_with_outputs_in_order() {
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(targets(vec![], vec![], None).unwrap(), [(Format::Text, None)]);
        assert_eq!(targets(vec![Format::Json], vec![], Some(p("c.md"))).unwrap(), [(Format::Json, None), (Format::Markdown, Some(p("c.md")))]);
        assert_eq!(
            targets(vec![Format::Sarif, Format::Text], vec![p("a.sarif"), p("-")], None).unwrap(),
            [(Format::Sarif, Some(p("a.sarif"))), (Format::Text, None)]
        );
        assert!(targets(vec![Format::Sarif, Format::Junit], vec![p("a")], None).is_err());
        assert!(targets(vec![], vec![p("a")], None).is_err());
    }

    #[test]
    fn the_cli_is_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
