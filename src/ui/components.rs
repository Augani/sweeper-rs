use crate::categories::FileCategory;
use adabraka_ui::prelude::*;

pub fn category_badge(category: FileCategory) -> Badge {
    Badge::new(category.name()).variant(match category {
        FileCategory::DevArtifact => BadgeVariant::Default,
        FileCategory::PackageCache => BadgeVariant::Secondary,
        FileCategory::IdeCache => BadgeVariant::Secondary,
        FileCategory::BrowserCache => BadgeVariant::Outline,
        FileCategory::SystemCache => BadgeVariant::Outline,
        FileCategory::LogFile => BadgeVariant::Secondary,
        FileCategory::TempFile => BadgeVariant::Destructive,
        FileCategory::LargeFile => BadgeVariant::Default,
        FileCategory::OldDownload => BadgeVariant::Secondary,
        FileCategory::Duplicate => BadgeVariant::Destructive,
        FileCategory::Unused => BadgeVariant::Outline,
    })
}

pub fn confidence_badge(confidence: f32) -> Badge {
    let percent = (confidence * 100.0) as u8;
    let label = format!("{}%", percent);

    let variant = if percent >= 90 {
        BadgeVariant::Default
    } else if percent >= 80 {
        BadgeVariant::Secondary
    } else {
        BadgeVariant::Outline
    };

    Badge::new(label).variant(variant)
}

pub fn size_text(size: u64) -> String {
    bytesize::ByteSize(size).to_string()
}
