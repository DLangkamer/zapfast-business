//! Dynamic template variables for messages, canned quick replies, and bulk dispatches.

use jiff::Zoned;

/// Contact information context used to substitute dynamic variables in templates.
#[derive(Clone, Debug, Default)]
pub struct ContactContext<'a> {
    pub full_name: Option<&'a str>,
    pub first_name: Option<&'a str>,
    pub phone: Option<&'a str>,
}

impl<'a> ContactContext<'a> {
    pub fn new(
        full_name: Option<&'a str>,
        first_name: Option<&'a str>,
        phone: Option<&'a str>,
    ) -> Self {
        Self {
            full_name,
            first_name,
            phone,
        }
    }
}

/// Resolves greeting ("Bom dia", "Boa tarde", "Boa noite") based on current local time.
pub fn greeting_now() -> &'static str {
    greeting_for_hour(Zoned::now().hour())
}

/// Returns greeting for an hour (0..=23).
pub fn greeting_for_hour(hour: i8) -> &'static str {
    if (5..12).contains(&hour) {
        "Bom dia"
    } else if (12..18).contains(&hour) {
        "Boa tarde"
    } else {
        "Boa noite"
    }
}

/// Returns the formatted date string "DD/MM/AAAA".
pub fn date_now() -> String {
    let now = Zoned::now();
    format!("{:02}/{:02}/{:04}", now.day(), now.month(), now.year())
}

/// Returns the formatted time string "HH:MM".
pub fn time_now() -> String {
    let now = Zoned::now();
    format!("{:02}:{:02}", now.hour(), now.minute())
}

/// Returns the weekday name in Portuguese.
pub fn weekday_now() -> &'static str {
    match Zoned::now().weekday() {
        jiff::civil::Weekday::Monday => "segunda-feira",
        jiff::civil::Weekday::Tuesday => "terça-feira",
        jiff::civil::Weekday::Wednesday => "quarta-feira",
        jiff::civil::Weekday::Thursday => "quinta-feira",
        jiff::civil::Weekday::Friday => "sexta-feira",
        jiff::civil::Weekday::Saturday => "sábado",
        jiff::civil::Weekday::Sunday => "domingo",
    }
}

/// Extracts the first name from a full name, ignoring phone numbers or empty strings.
pub fn extract_first_name(full_name: &str) -> &str {
    let trimmed = full_name.trim();
    if trimmed.is_empty() {
        return "";
    }
    if trimmed.starts_with('+') {
        return trimmed;
    }
    trimmed.split_whitespace().next().unwrap_or(trimmed)
}

/// Replaces all known dynamic variables in `template`.
///
/// Supported variables (case-insensitive, with either `{{tag}}` or `{tag}`):
/// - `{{primeiro_nome}}`, `{{primeironome}}`, `{{primeiro nome}}`: Contact's first name
/// - `{{nome}}`: Contact's full or saved name
/// - `{{saudacao}}`: "Bom dia", "Boa tarde", or "Boa noite"
/// - `{{data}}`: Current date (e.g. "06/10/2026")
/// - `{{hora}}`: Current time (e.g. "08:30")
/// - `{{dia_semana}}`, `{{dia}}`: Day of the week in Portuguese (e.g. "segunda-feira")
/// - `{{telefone}}`, `{{numero}}`: Contact's phone number
pub fn resolve_template(template: &str, context: &ContactContext<'_>) -> String {
    if !template.contains('{') {
        return template.to_owned();
    }

    let greeting = greeting_now();
    let date = date_now();
    let time = time_now();
    let weekday = weekday_now();

    let full_name = context.full_name.unwrap_or("").trim();
    let first_name = context
        .first_name
        .unwrap_or_else(|| extract_first_name(full_name))
        .trim();
    let phone = context.phone.unwrap_or("").trim();

    let effective_full_name = if !full_name.is_empty() {
        full_name
    } else {
        first_name
    };

    let replacements: [(&[&str], &str); 7] = [
        (
            &[
                "{{primeiro_nome}}",
                "{{primeironome}}",
                "{{primeiro nome}}",
                "{primeiro_nome}",
                "{primeironome}",
            ],
            first_name,
        ),
        (&["{{nome}}", "{nome}"], effective_full_name),
        (&["{{saudacao}}", "{saudacao}"], greeting),
        (&["{{data}}", "{data}"], &date),
        (&["{{hora}}", "{hora}"], &time),
        (
            &["{{dia_semana}}", "{{dia}}", "{dia_semana}", "{dia}"],
            weekday,
        ),
        (
            &["{{telefone}}", "{{numero}}", "{telefone}", "{numero}"],
            phone,
        ),
    ];

    let mut result = template.to_owned();
    for (tags, value) in replacements {
        for &tag in tags {
            result = replace_case_insensitive(&result, tag, value);
        }
    }

    result
}

fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower_haystack = haystack.to_lowercase();
    let lower_needle = needle.to_lowercase();
    let needle_len = needle.len();

    let mut out = String::with_capacity(haystack.len());
    let mut last_idx = 0;

    for (start, _) in lower_haystack.match_indices(&lower_needle) {
        if start < last_idx {
            continue;
        }
        out.push_str(&haystack[last_idx..start]);
        out.push_str(replacement);
        last_idx = start + needle_len;
    }
    out.push_str(&haystack[last_idx..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_first_names() {
        assert_eq!(extract_first_name("Douglas Langkamer"), "Douglas");
        assert_eq!(extract_first_name("Maria"), "Maria");
        assert_eq!(extract_first_name(""), "");
        assert_eq!(extract_first_name("+55 31 9999-9999"), "+55 31 9999-9999");
    }

    #[test]
    fn greeting_brackets() {
        assert_eq!(greeting_for_hour(8), "Bom dia");
        assert_eq!(greeting_for_hour(14), "Boa tarde");
        assert_eq!(greeting_for_hour(20), "Boa noite");
        assert_eq!(greeting_for_hour(2), "Boa noite");
    }

    #[test]
    fn resolves_template_variables() {
        let ctx = ContactContext::new(
            Some("Douglas Langkamer"),
            Some("Douglas"),
            Some("5531999998888"),
        );
        let template = "Olá {{primeiro_nome}}, {{saudacao}}! Seu telefone é {{telefone}}.";
        let resolved = resolve_template(template, &ctx);
        assert!(resolved.starts_with("Olá Douglas, "));
        assert!(resolved.ends_with("! Seu telefone é 5531999998888."));
    }

    #[test]
    fn resolves_single_and_double_braces_case_insensitive() {
        let ctx = ContactContext::new(Some("Douglas"), None, None);
        let template = "Oi {Primeiro_Nome}, {NOME}!";
        let resolved = resolve_template(template, &ctx);
        assert_eq!(resolved, "Oi Douglas, Douglas!");
    }
}
