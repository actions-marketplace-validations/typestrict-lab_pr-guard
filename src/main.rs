use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use pr_guard::{diff, model, policy::Policy, report, rules, verdict};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // parsed once
enum Cmd {
    /// Decide a pull request: the policy over the diff and TS-PRGuard's answers.
    Check {
        #[arg(long)]
        diff: PathBuf,
        /// TS-PRGuard's answers as JSON, instead of running the model.
        #[arg(long)]
        answers: Option<PathBuf>,
        /// The pull request's description (title and body); without it the description question is not asked.
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
        /// The pull request's labels (repeat the flag).
        #[arg(long = "label")]
        labels: Vec<String>,
        /// Where to write the sticky comment's markdown.
        #[arg(long)]
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
        Cmd::Check { diff: d, answers, description, description_file, no_model, model: model_dir, threads, policy, labels, comment } => {
            let policy = match std::fs::read_to_string(&policy) {
                Ok(s) => Policy::from_yaml(&s).with_context(|| format!("reading {}", policy.display()))?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Policy::default(),
                Err(e) => return Err(e).context(format!("reading {}", policy.display())),
            };
            // As the lab mined it (`errors="replace"`): invalid UTF-8 becomes U+FFFD instead of an error.
            let raw = String::from_utf8_lossy(&std::fs::read(&d).with_context(|| format!("reading {}", d.display()))?).into_owned();
            let files = diff::parse(&raw);
            let description = match description_file {
                Some(p) => Some(std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?),
                None => description,
            };
            let mut answers = match answers {
                Some(p) => serde_json::from_str(&std::fs::read_to_string(&p)?).with_context(|| format!("reading {}", p.display()))?,
                None => verdict::Answers::default(),
            };
            if !no_model && answers == verdict::Answers::default() {
                ask_model(&mut answers, model_dir, threads, description.as_deref(), &raw)?;
            }
            let found = rules::check(&policy.rules, &files)?;
            let v = verdict::decide(&policy, &files, &answers, &labels, found);
            print!("{}", report::annotations(&v));
            if let Some(p) = comment {
                std::fs::write(p, report::comment(&v))?;
            }
            println!("{}", serde_json::to_string(&v)?);
            if v.blocked {
                std::process::exit(1);
            }
        }
    }
    Ok(())
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
