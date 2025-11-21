use crate::{errors::AppError, output::dependency::DependencyKind};

#[derive(Debug, Clone, Copy)]
pub enum MarkdownSection<'a> {
    Header(&'a str),
    Item(ListEntry<'a>),
}

pub struct MarkdownListTokenizer<'a> {
    lines: std::str::Lines<'a>,
}

impl<'a> MarkdownListTokenizer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            lines: content.lines(),
        }
    }
}

impl<'a> Iterator for MarkdownListTokenizer<'a> {
    type Item = Result<MarkdownSection<'a>, AppError>;

    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("## ") {
                return Some(Ok(MarkdownSection::Header(trimmed)));
            }

            if trimmed.starts_with('-') {
                return Some(ListEntry::from_line(trimmed).map(MarkdownSection::Item));
            }
        }

        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ListEntry<'a> {
    pub raw_line: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub crate_segment: &'a str,
    pub source_segment: &'a str,
    pub stats_segment: &'a str,
    pub status_segment: &'a str,
}

impl<'a> ListEntry<'a> {
    pub fn from_line(line: &'a str) -> Result<Self, AppError> {
        let content = line.trim_start_matches('-').trim();
        let (name_part, remainder) = content
            .split_once(" : ")
            .ok_or_else(|| AppError::InvalidListLine(line.to_string()))?;
        let name = name_part.trim();
        let (description_part, tail) = remainder
            .split_once(" - ")
            .ok_or_else(|| AppError::InvalidListLine(line.to_string()))?;
        let description = match description_part.trim() {
            "" => None,
            text => Some(text),
        };

        let mut rest = tail.trim();
        let (crate_segment, leftover) = take_markdown_link(rest, line)?;
        rest = leftover;
        let (source_segment, leftover) = take_markdown_link(rest, line)?;
        rest = leftover;
        let (stats_segment, leftover) = take_parenthesized_segment(rest, line)?;
        rest = leftover.trim();
        if rest.is_empty() {
            return Err(AppError::InvalidListLine(line.to_string()));
        }

        Ok(Self {
            raw_line: line,
            name,
            description,
            crate_segment,
            source_segment,
            stats_segment,
            status_segment: rest,
        })
    }
}

fn take_markdown_link<'a>(input: &'a str, line: &str) -> Result<(&'a str, &'a str), AppError> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('[') {
        return Err(AppError::InvalidListLine(line.to_string()));
    }

    let mut closing_bracket = None;
    for (idx, ch) in trimmed.char_indices() {
        if ch == ']' {
            closing_bracket = Some(idx);
            break;
        }
    }
    let closing_bracket =
        closing_bracket.ok_or_else(|| AppError::InvalidListLine(line.to_string()))?;
    let after_bracket = &trimmed[closing_bracket + 1..];
    if !after_bracket.starts_with('(') {
        return Err(AppError::InvalidListLine(line.to_string()));
    }

    let mut depth = 0i32;
    let mut closing_paren = None;
    for (offset, ch) in after_bracket.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    closing_paren = Some(offset);
                    break;
                }
            }
            _ => {}
        }
    }

    let closing_paren = closing_paren.ok_or_else(|| AppError::InvalidListLine(line.to_string()))?;
    let segment_len = closing_bracket + 1 + closing_paren + 1;
    let segment = &trimmed[..segment_len + 1];
    let remainder = after_bracket[closing_paren + 1..].trim_start();
    Ok((segment, remainder))
}

fn take_parenthesized_segment<'a>(
    input: &'a str,
    line: &str,
) -> Result<(&'a str, &'a str), AppError> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('(') {
        return Err(AppError::InvalidListLine(line.to_string()));
    }

    let mut depth = 0i32;
    let mut closing_paren = None;
    for (idx, ch) in trimmed.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    closing_paren = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }

    let closing_paren = closing_paren.ok_or_else(|| AppError::InvalidListLine(line.to_string()))?;
    let segment = &trimmed[..=closing_paren];
    let remainder = trimmed[closing_paren + 1..].trim_start();
    Ok((segment, remainder))
}

pub fn section_kind_from_header(header: &str) -> Option<DependencyKind> {
    DependencyKind::try_from_list_header_line(header).ok()
}
