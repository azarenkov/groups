pub const PREVIEW_SOURCE_MAX_BYTES: i64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewSource {
    Original,
    Rendition,
}

const PDF: &str = "application/pdf";

const OFFICE_TYPES: &[(&str, &str)] = &[
    ("application/msword", "doc"),
    ("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "docx"),
    ("application/vnd.oasis.opendocument.text", "odt"),
    ("application/rtf", "rtf"),
    ("text/rtf", "rtf"),
    ("application/vnd.ms-powerpoint", "ppt"),
    ("application/vnd.openxmlformats-officedocument.presentationml.presentation", "pptx"),
    ("application/vnd.oasis.opendocument.presentation", "odp"),
    ("application/vnd.ms-excel", "xls"),
    ("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "xlsx"),
    ("application/vnd.oasis.opendocument.spreadsheet", "ods"),
];

const OFFICE_EXTENSIONS: &[&str] = &["doc", "docx", "odt", "rtf", "ppt", "pptx", "odp", "xls", "xlsx", "ods"];

fn base_type(content_type: &str) -> String {
    content_type.split(';').next().unwrap_or_default().trim().to_ascii_lowercase()
}

fn extension(file_name: &str) -> Option<String> {
    let (stem, ext) = file_name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty()).then(|| ext.to_ascii_lowercase())
}

fn office_extension(content_type: &str, file_name: &str) -> Option<String> {
    let base = base_type(content_type);
    if let Some((_, ext)) = OFFICE_TYPES.iter().find(|(ty, _)| *ty == base) {
        return Some((*ext).to_string());
    }
    extension(file_name).filter(|ext| OFFICE_EXTENSIONS.contains(&ext.as_str()))
}

pub fn preview_source(content_type: &str, file_name: &str) -> Option<PreviewSource> {
    if base_type(content_type) == PDF || extension(file_name).as_deref() == Some("pdf") {
        return Some(PreviewSource::Original);
    }
    office_extension(content_type, file_name).map(|_| PreviewSource::Rendition)
}

pub fn rendition_input_name(content_type: &str, file_name: &str) -> Option<String> {
    let ext = office_extension(content_type, file_name)?;
    Some(format!("document.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_is_shown_as_is_and_office_files_need_a_rendition() {
        assert_eq!(preview_source("application/pdf", "a.pdf"), Some(PreviewSource::Original));
        assert_eq!(preview_source("application/octet-stream", "Report.PDF"), Some(PreviewSource::Original));
        assert_eq!(
            preview_source("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "a.docx"),
            Some(PreviewSource::Rendition)
        );
        assert_eq!(preview_source("application/octet-stream", "slides.pptx"), Some(PreviewSource::Rendition));
        assert_eq!(preview_source("application/vnd.ms-excel; charset=binary", "sheet"), Some(PreviewSource::Rendition));
        assert_eq!(preview_source("image/png", "a.png"), None);
        assert_eq!(preview_source("application/zip", "a.zip"), None);
        assert_eq!(preview_source("text/plain", ".docx"), None);
    }

    #[test]
    fn the_converter_gets_a_neutral_name_with_the_right_extension() {
        assert_eq!(rendition_input_name("application/vnd.ms-powerpoint", "x"), Some("document.ppt".to_string()));
        assert_eq!(rendition_input_name("application/octet-stream", "Отчёт \"final\".XLSX"), Some("document.xlsx".to_string()));
        assert_eq!(rendition_input_name("application/pdf", "a.pdf"), None);
    }
}
