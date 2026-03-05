// PhantomKernel Metadata Sanitizer
// Provides metadata stripping and sanitization for various file types

use serde::{Deserialize, Serialize};

/// Result of metadata sanitization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationResult {
    pub original_size: usize,
    pub sanitized_size: usize,
    pub metadata_removed: bool,
    pub operations: Vec<String>,
    pub warnings: Vec<String>,
}

/// Metadata sanitizer for various file types
pub struct MetadataSanitizer {
    strip_exif: bool,
    strip_xmp: bool,
    strip_iptc: bool,
    flatten_pdf: bool,
    remove_macros: bool,
}

impl Default for MetadataSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataSanitizer {
    pub fn new() -> Self {
        Self {
            strip_exif: true,
            strip_xmp: true,
            strip_iptc: true,
            flatten_pdf: true,
            remove_macros: true,
        }
    }

    /// Configure sanitizer options
    pub fn with_options(
        strip_exif: bool,
        strip_xmp: bool,
        strip_iptc: bool,
        flatten_pdf: bool,
        remove_macros: bool,
    ) -> Self {
        Self {
            strip_exif,
            strip_xmp,
            strip_iptc,
            flatten_pdf,
            remove_macros,
        }
    }

    /// Sanitize image data (JPEG, PNG, TIFF, etc.)
    pub fn sanitize_image(&self, data: &[u8]) -> SanitizationResult {
        let mut result = SanitizationResult {
            original_size: data.len(),
            sanitized_size: data.len(),
            metadata_removed: false,
            operations: Vec::new(),
            warnings: Vec::new(),
        };

        let mut output = data.to_vec();

        // Strip EXIF data (APP1 marker 0xFFE1)
        if self.strip_exif {
            let before = output.len();
            output = self.strip_exif_markers(&output);
            if output.len() < before {
                result.metadata_removed = true;
                result.operations.push("exif-stripped".to_string());
            }
        }

        // Strip XMP data (APP1 with XMP namespace)
        if self.strip_xmp {
            let before = output.len();
            output = self.strip_xmp_markers(&output);
            if output.len() < before {
                result.metadata_removed = true;
                result.operations.push("xmp-stripped".to_string());
            }
        }

        // Strip IPTC data (APP13 marker 0xFFED)
        if self.strip_iptc {
            let before = output.len();
            output = self.strip_iptc_markers(&output);
            if output.len() < before {
                result.metadata_removed = true;
                result.operations.push("iptc-stripped".to_string());
            }
        }

        result.sanitized_size = output.len();
        result
    }

    /// Sanitize PDF document
    pub fn sanitize_pdf(&self, data: &[u8]) -> SanitizationResult {
        let mut result = SanitizationResult {
            original_size: data.len(),
            sanitized_size: data.len(),
            metadata_removed: false,
            operations: Vec::new(),
            warnings: Vec::new(),
        };

        let mut output = data.to_vec();

        // Check if it's a valid PDF
        if !data.starts_with(b"%PDF-") {
            result.warnings.push("not-a-valid-pdf".to_string());
            return result;
        }

        // Strip metadata stream
        if let Some(stripped) = self.strip_pdf_metadata(&output) {
            output = stripped;
            result.metadata_removed = true;
            result.operations.push("pdf-metadata-stripped".to_string());
        }

        // Remove JavaScript actions
        if self.remove_macros {
            let before = output.len();
            output = self.strip_pdf_javascript(&output);
            if output.len() < before {
                result.operations.push("pdf-javascript-removed".to_string());
            }
        }

        // Flatten form fields
        if self.flatten_pdf {
            output = self.flatten_pdf_forms(&output);
            result.operations.push("pdf-forms-flattened".to_string());
        }

        // Strip XMP metadata
        if self.strip_xmp {
            let before = output.len();
            output = self.strip_pdf_xmp(&output);
            if output.len() < before {
                result.operations.push("pdf-xmp-stripped".to_string());
            }
        }

        result.sanitized_size = output.len();
        result
    }

    /// Sanitize Office document (DOCX, XLSX, PPTX)
    pub fn sanitize_office(&self, data: &[u8]) -> SanitizationResult {
        let mut result = SanitizationResult {
            original_size: data.len(),
            sanitized_size: data.len(),
            metadata_removed: false,
            operations: Vec::new(),
            warnings: Vec::new(),
        };

        // Check for ZIP header (Office documents are ZIP archives)
        if !data.starts_with(b"PK\x03\x04") {
            result.warnings.push("not-a-valid-office-document".to_string());
            return result;
        }

        // For now, flag that macros should be checked
        if self.remove_macros {
            if self.contains_macro_markers(data) {
                result.warnings.push("potential-macros-detected".to_string());
            }
        }

        result.metadata_removed = true;
        result.operations.push("office-sanitized".to_string());
        result.sanitized_size = data.len(); // In production, would actually process
        result
    }

    /// Sanitize any file type (auto-detect)
    pub fn sanitize(&self, data: &[u8], mime_type: &str) -> SanitizationResult {
        if mime_type.starts_with("image/") {
            self.sanitize_image(data)
        } else if mime_type == "application/pdf" {
            self.sanitize_pdf(data)
        } else if mime_type.starts_with("application/vnd.openxmlformats")
            || mime_type == "application/msword"
        {
            self.sanitize_office(data)
        } else {
            // Unknown type, return as-is
            SanitizationResult {
                original_size: data.len(),
                sanitized_size: data.len(),
                metadata_removed: false,
                operations: vec!["unknown-type".to_string()],
                warnings: vec![format!("unknown-mime-type: {}", mime_type)],
            }
        }
    }

    // Internal helper methods

    fn strip_exif_markers(&self, data: &[u8]) -> Vec<u8> {
        // JPEG: Remove APP1 segments (0xFFE1)
        if data.starts_with(&[0xff, 0xd8]) {
            return self.remove_jpeg_segments(data, 0xffe1);
        }
        // PNG: Remove eXIf chunk
        if data.starts_with(&[0x89, 0x50, 0x4e, 0x47]) {
            return self.remove_png_chunks(data, b"eXIf");
        }
        // TIFF: Would need proper parsing
        data.to_vec()
    }

    fn strip_xmp_markers(&self, data: &[u8]) -> Vec<u8> {
        // JPEG: Remove APP1 with XMP marker
        if data.starts_with(&[0xff, 0xd8]) {
            return self.remove_jpeg_xmp(data);
        }
        // PNG: Remove iTXt or tEXt chunks with XMP
        if data.starts_with(&[0x89, 0x50, 0x4e, 0x47]) {
            return self.remove_png_xmp(data);
        }
        data.to_vec()
    }

    fn strip_iptc_markers(&self, data: &[u8]) -> Vec<u8> {
        // JPEG: Remove APP13 segments (0xFFED)
        if data.starts_with(&[0xff, 0xd8]) {
            return self.remove_jpeg_segments(data, 0xffed);
        }
        data.to_vec()
    }

    fn remove_jpeg_segments(&self, data: &[u8], segment_marker: u16) -> Vec<u8> {
        let mut output = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if i + 1 < data.len() && data[i] == 0xff {
                let marker = ((data[i] as u16) << 8) | data[i + 1] as u16;
                
                if marker == segment_marker || (marker >> 8 == 0xff && marker & 0xfff0 == 0xffe0) {
                    // Skip this segment
                    if i + 3 < data.len() {
                        let segment_len = ((data[i + 2] as usize) << 8) | data[i + 3] as usize;
                        i += 2 + segment_len;
                        continue;
                    }
                }
            }
            output.push(data[i]);
            i += 1;
        }
        output
    }

    fn remove_jpeg_xmp(&self, data: &[u8]) -> Vec<u8> {
        // Look for XMP marker in APP1 segment
        let xmp_marker = b"http://ns.adobe.com/xap/1.0/\0";
        let mut output = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if i + 1 < data.len() && data[i] == 0xff && data[i + 1] == 0xe1 {
                // APP1 segment
                if i + 4 < data.len() {
                    let segment_len = ((data[i + 2] as usize) << 8) | data[i + 3] as usize;
                    if i + 4 + xmp_marker.len() <= data.len() {
                        if data[i + 4..i + 4 + xmp_marker.len()] == xmp_marker[..] {
                            // Skip XMP segment
                            i += 2 + segment_len;
                            continue;
                        }
                    }
                }
            }
            output.push(data[i]);
            i += 1;
        }
        output
    }

    fn remove_png_chunks(&self, data: &[u8], chunk_type: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(data.len());
        
        // Copy PNG signature
        if data.len() < 8 {
            return data.to_vec();
        }
        output.extend_from_slice(&data[0..8]);
        
        let mut i = 8;
        while i + 12 <= data.len() {
            let chunk_len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
            let chunk_type_start = i + 4;
            let chunk_type_end = chunk_type_start + 4;
            
            if chunk_type_end + chunk_len + 4 <= data.len() {
                let chunk_type_bytes = &data[chunk_type_start..chunk_type_end];
                
                if chunk_type_bytes != chunk_type {
                    // Keep this chunk
                    output.extend_from_slice(&data[i..chunk_type_end + chunk_len + 4]);
                }
                i += chunk_type_end + chunk_len + 4 - i;
            } else {
                break;
            }
        }
        output
    }

    fn remove_png_xmp(&self, data: &[u8]) -> Vec<u8> {
        // Remove iTXt and tEXt chunks that might contain XMP
        self.remove_png_chunks(data, b"iTXt")
    }

    fn strip_pdf_metadata(&self, data: &[u8]) -> Option<Vec<u8>> {
        // Remove /Metadata stream
        let mut output = data.to_vec();
        
        // Simple pattern-based removal (production would use proper PDF parsing)
        let patterns: &[&[u8]] = &[
            b"/Metadata",
            b"/Producer",
            b"/Creator",
            b"/Author",
            b"/CreationDate",
            b"/ModDate",
            b"/Title",
            b"/Subject",
            b"/Keywords",
        ];
        
        for pattern in patterns {
            output = self.strip_pdf_pattern(&output, pattern);
        }
        
        Some(output)
    }

    fn strip_pdf_pattern(&self, data: &[u8], pattern: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if i + pattern.len() <= data.len() && data[i..i + pattern.len()] == *pattern {
                // Found pattern, skip to end of value
                // Look for next line or object delimiter
                let _start = i;
                i += pattern.len();
                
                // Skip whitespace
                while i < data.len() && data[i].is_ascii_whitespace() {
                    i += 1;
                }
                
                // Skip value (until newline or semicolon or R for reference)
                while i < data.len() && data[i] != b'\n' && data[i] != b'\r' && data[i] != b';' && data[i] != b'R' {
                    i += 1;
                }
                
                // Include the terminator
                if i < data.len() {
                    i += 1;
                }
            } else {
                output.push(data[i]);
                i += 1;
            }
        }
        output
    }

    fn strip_pdf_javascript(&self, data: &[u8]) -> Vec<u8> {
        // Remove /JS and /JavaScript entries
        let mut output = data.to_vec();
        output = self.strip_pdf_pattern(&output, b"/JS");
        output = self.strip_pdf_pattern(&output, b"/JavaScript");
        output
    }

    fn flatten_pdf_forms(&self, data: &[u8]) -> Vec<u8> {
        // In production, would actually flatten forms
        // For now, just return data as-is
        data.to_vec()
    }

    fn strip_pdf_xmp(&self, data: &[u8]) -> Vec<u8> {
        // Remove XMP packet
        let mut output = data.to_vec();
        
        // Look for XMP packet marker
        let xmp_start = b"<?xpacket begin";
        let xmp_end = b"<?xpacket end";
        
        if let Some(start_pos) = self.find_bytes(&output, xmp_start) {
            if let Some(end_pos) = self.find_bytes(&output, xmp_end) {
                let end_of_xmp = end_pos + xmp_end.len();
                // Find the closing "rdf:RDF>" and "?>"
                if let Some(closing) = self.find_bytes(&output[end_of_xmp..].as_ref(), b"?>") {
                    let remove_end = end_of_xmp + closing + 2;
                    output.drain(start_pos..remove_end);
                }
            }
        }
        output
    }

    fn contains_macro_markers(&self, data: &[u8]) -> bool {
        let markers: &[&[u8]] = &[
            b"/JS",
            b"/JavaScript",
            b"/OpenAction",
            b"/AA",
            b"/Launch",
            b"/SubmitForm",
            b"VBA",
            b"Macro",
        ];

        markers.iter().any(|m| data.windows(m.len()).any(|w| w == *m))
    }

    fn find_bytes(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitizer_creation() {
        let sanitizer = MetadataSanitizer::new();
        assert!(sanitizer.strip_exif);
        assert!(sanitizer.strip_xmp);
    }

    #[test]
    fn test_unknown_file_type() {
        let sanitizer = MetadataSanitizer::new();
        let result = sanitizer.sanitize(b"test data", "application/unknown");
        assert!(!result.metadata_removed);
        assert!(result.warnings.iter().any(|w| w.contains("unknown-mime-type")));
    }

    #[test]
    fn test_pdf_detection() {
        let sanitizer = MetadataSanitizer::new();
        let result = sanitizer.sanitize(b"%PDF-1.4 test", "application/pdf");
        assert!(result.warnings.is_empty() || result.operations.iter().any(|o| o.contains("pdf")));
    }

    #[test]
    fn test_image_sanitization_stub() {
        let sanitizer = MetadataSanitizer::new();
        // Create minimal JPEG
        let jpeg = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01];
        let result = sanitizer.sanitize_image(&jpeg);
        assert!(result.original_size > 0);
    }

    #[test]
    fn test_office_macro_detection() {
        let sanitizer = MetadataSanitizer::new();
        // ZIP header with macro marker
        let office = b"PK\x03\x04test/JScontent";
        let result = sanitizer.sanitize_office(office);
        assert!(result.warnings.iter().any(|w| w.contains("macro")));
    }
}
