#[cfg(test)]
mod tests {
    use std::fs;
    use crate::jpeg;

    #[test]
    fn test_all_images() {
        let image_paths = fs::read_dir("test_images/").unwrap();

        for path in image_paths {
            let data = fs::read(path.unwrap().path()).unwrap();
            jpeg::read_jpeg(data).unwrap();
        }
    }
}



