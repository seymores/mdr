use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct CliArgs {
    pub enable_beeline: bool,
    pub inputs: Vec<PathBuf>,
}

pub fn parse_args<I, S>(args: I) -> Result<CliArgs, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut enable_beeline = true;
    let mut inputs = Vec::new();

    for arg in args.into_iter().skip(1) {
        let arg = arg.as_ref();
        if arg == "--no-beeline" {
            enable_beeline = false;
        } else {
            inputs.push(PathBuf::from(arg));
        }
    }

    if inputs.is_empty() {
        return Err(
            "Usage: mdr [--no-beeline] <path-to-markdown> [more paths or directories]".to_string(),
        );
    }

    Ok(CliArgs {
        enable_beeline,
        inputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_beeline_and_multiple_inputs() {
        let parsed = parse_args(["mdr", "--no-beeline", "a.md", "docs"]).unwrap();
        assert!(!parsed.enable_beeline);
        assert_eq!(
            parsed.inputs,
            vec![PathBuf::from("a.md"), PathBuf::from("docs")]
        );
    }

    #[test]
    fn returns_usage_error_when_no_inputs() {
        let err = parse_args(["mdr"]).unwrap_err();
        assert!(err.contains("Usage: mdr"));
    }
}
