#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    values: Vec<String>,
}

impl Tags {
    pub fn parse(value: &str) -> Self {
        Self {
            values: value
                .split(',')
                .filter_map(|tag| {
                    let tag = tag.trim();
                    (!tag.is_empty()).then(|| tag.to_owned())
                })
                .collect(),
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = String>) -> Self {
        Self {
            values: values
                .into_iter()
                .filter_map(|tag| {
                    let tag = tag.trim().to_owned();
                    (!tag.is_empty()).then_some(tag)
                })
                .collect(),
        }
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn merged(&self, tags: &Self) -> Self {
        let mut values = self.values.clone();

        for tag in &tags.values {
            if !values
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(tag))
            {
                values.push(tag.clone());
            }
        }

        Self { values }
    }

    pub fn without(&self, tag: &str) -> Self {
        Self::from_values(
            self.values
                .iter()
                .filter(|existing| existing.as_str() != tag)
                .cloned(),
        )
    }

    pub fn as_text(&self) -> String {
        self.values.join(", ")
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();

        !query.is_empty()
            && self
                .values
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query))
    }
}
