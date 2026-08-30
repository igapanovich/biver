use crate::error::{Error, Result};
use std::process::{Command, Output};

pub fn run_templated_command<StrIter, S>(
    command_template: StrIter,
    template_values: &[(&str, &str)],
) -> Result<()>
where
    StrIter: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut command_template = command_template.into_iter();

    let Some(command) = command_template.next() else {
        return Err(Error::ExternalCommand(
            "Empty templated command".to_string(),
        ));
    };

    let mut command = Command::new(command.as_ref().to_string());

    loop {
        let Some(arg) = command_template.next() else {
            break;
        };

        if let Some(rendered_arg) = parse_and_render_template(arg.as_ref(), template_values) {
            command.arg(rendered_arg);
        } else {
            command.arg(arg.as_ref());
        };
    }

    let output = command.output()?;

    if output.status.success() {
        return Ok(());
    }

    process_output_to_result(output)
}

fn parse_and_render_template(template: &str, template_values: &[(&str, &str)]) -> Option<String> {
    let template = parse_template(template)?;
    render_template(template, template_values)
}

fn render_template(
    template: Vec<TemplatePart>,
    template_values: &[(&str, &str)],
) -> Option<String> {
    let mut result = String::new();

    for part in template {
        match part {
            TemplatePart::Literal(l) => result.push_str(l),
            TemplatePart::Variable(name) => {
                let Some(value) = template_values
                    .iter()
                    .filter_map(|(k, v)| if k == &name { Some(v) } else { None })
                    .next()
                else {
                    return None;
                };

                result.push_str(&value);
            }
        }
    }

    Some(result)
}

#[derive(Debug, PartialEq, Eq)]
enum TemplatePart<'a> {
    Literal(&'a str),
    Variable(&'a str),
}

fn parse_template(template: &str) -> Option<Vec<TemplatePart<'_>>> {
    let Some((start, end)) = find_square_bracket_seg(template, 0) else {
        return None;
    };

    let mut result = Vec::new();

    if start > 0 {
        result.push(TemplatePart::Literal(&template[0..start]));
    }

    result.push(TemplatePart::Variable(&template[start + 1..end]));

    let mut offset = end + 1;

    loop {
        if let Some((start, end)) = find_square_bracket_seg(template, offset) {
            if start > offset {
                result.push(TemplatePart::Literal(&template[offset..start]));
            }

            result.push(TemplatePart::Variable(&template[start + 1..end]));

            offset = end + 1;
        } else {
            if offset < template.len() {
                result.push(TemplatePart::Literal(&template[offset..]));
            }

            return Some(result);
        }
    }
}

fn find_square_bracket_seg(str: &str, start_index: usize) -> Option<(usize, usize)> {
    enum State {
        NothingFound,
        FoundStart(usize),
    }

    let mut state = State::NothingFound;

    for (index, char) in str.char_indices() {
        if index < start_index {
            continue;
        }

        if char == '{' {
            state = State::FoundStart(index);
        }
        if char == '}' {
            match state {
                State::NothingFound => {}
                State::FoundStart(start) => return Some((start, index)),
            }
        }
    }

    None
}

fn process_output_to_result(output: Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let mut message = format!("command exited with {}", output.status);

    if !output.stdout.is_empty() {
        message.push_str("\nstdout:\n");
        message.push_str(&String::from_utf8(output.stdout)?);
    }

    if !output.stderr.is_empty() {
        message.push_str("\nstderr:\n");
        message.push_str(&String::from_utf8(output.stderr)?);
    }

    Err(Error::ExternalCommand(message).into())
}

#[cfg(test)]
mod tests {
    use crate::external_command::{TemplatePart, parse_template, render_template};

    #[test]
    fn parse_template_is_correct() {
        assert_eq!(parse_template(""), None);

        assert_eq!(
            parse_template("{x}"),
            Some(vec![TemplatePart::Variable("x")])
        );

        assert_eq!(
            parse_template("a{x}"),
            Some(vec![
                TemplatePart::Literal("a"),
                TemplatePart::Variable("x"),
            ])
        );

        assert_eq!(
            parse_template("{x}a"),
            Some(vec![
                TemplatePart::Variable("x"),
                TemplatePart::Literal("a"),
            ])
        );

        assert_eq!(
            parse_template("a{x}b"),
            Some(vec![
                TemplatePart::Literal("a"),
                TemplatePart::Variable("x"),
                TemplatePart::Literal("b"),
            ])
        );

        assert_eq!(
            parse_template("{x}{y}"),
            Some(vec![
                TemplatePart::Variable("x"),
                TemplatePart::Variable("y"),
            ])
        );

        assert_eq!(
            parse_template("a{x}{y}"),
            Some(vec![
                TemplatePart::Literal("a"),
                TemplatePart::Variable("x"),
                TemplatePart::Variable("y"),
            ])
        );

        assert_eq!(
            parse_template("{x}a{y}"),
            Some(vec![
                TemplatePart::Variable("x"),
                TemplatePart::Literal("a"),
                TemplatePart::Variable("y"),
            ])
        );

        assert_eq!(
            parse_template("{x}{y}a"),
            Some(vec![
                TemplatePart::Variable("x"),
                TemplatePart::Variable("y"),
                TemplatePart::Literal("a"),
            ])
        );

        assert_eq!(
            parse_template("a{x}b{y}"),
            Some(vec![
                TemplatePart::Literal("a"),
                TemplatePart::Variable("x"),
                TemplatePart::Literal("b"),
                TemplatePart::Variable("y"),
            ])
        );

        assert_eq!(
            parse_template("a{x}{y}b"),
            Some(vec![
                TemplatePart::Literal("a"),
                TemplatePart::Variable("x"),
                TemplatePart::Variable("y"),
                TemplatePart::Literal("b"),
            ])
        );

        assert_eq!(
            parse_template("{x}a{y}b"),
            Some(vec![
                TemplatePart::Variable("x"),
                TemplatePart::Literal("a"),
                TemplatePart::Variable("y"),
                TemplatePart::Literal("b"),
            ])
        );

        assert_eq!(
            parse_template("a{x}b{y}c"),
            Some(vec![
                TemplatePart::Literal("a"),
                TemplatePart::Variable("x"),
                TemplatePart::Literal("b"),
                TemplatePart::Variable("y"),
                TemplatePart::Literal("c"),
            ])
        );
    }

    #[test]
    fn render_template_is_correct() {
        assert_eq!(
            render_template(
                vec![
                    TemplatePart::Literal("a"),
                    TemplatePart::Variable("x"),
                    TemplatePart::Literal("c")
                ],
                &[("x", "b")]
            ),
            Some("abc".to_string())
        )
    }
}
