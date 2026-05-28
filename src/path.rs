fn path_seqment_to_folder(segment: &str) -> String {
    if segment.starts_with('{') && segment.ends_with('}') {
        format!("_{}", &segment[1..segment.len() - 1])
    } else {
        segment.to_string()
    }
}

pub fn path_to_folders(path: &str) -> String {
    path.trim_start_matches('/')
        .split("/")
        .map(path_seqment_to_folder)
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_literal_seqment_when_converting_to_folder_then_unchanged() {
        assert_eq!(path_seqment_to_folder("users"), "users");
    }

    #[test]
    fn given_param_seqment_when_converting_to_folder_then_prefixed() {
        assert_eq!(path_seqment_to_folder("{id}"), "_id");
    }

    #[test]
    fn given_full_path_when_converting_to_folder_then_all_segments_converted() {
        assert_eq!(
            path_to_folders("/users/{id}/orders/{orderId}"),
            "users/_id/orders/_orderId"
        );
    }
}
