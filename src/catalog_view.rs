use crate::mvt_rs_client::{RemoteCategory, RemoteField, RemoteLayer, RemoteStyle};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FieldView {
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LayerView {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub category_id: String,
    pub category_name: String,
    pub geometry: String,
    pub published: bool,
    pub fields: Vec<FieldView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryView {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl From<RemoteCategory> for CategoryView {
    fn from(c: RemoteCategory) -> Self {
        CategoryView {
            id: c.id,
            name: c.name,
            description: c.description,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StyleView {
    pub id: String,
    pub name: String,
    pub category_id: String,
    pub category_name: String,
    pub description: String,
}

/// Lists existing styles without their (often large) `style` JSON body —
/// callers that need the full MapLibre style document use `update_style`'s
/// response or `build_style_detail_view` below, not this listing.
impl From<RemoteStyle> for StyleView {
    fn from(s: RemoteStyle) -> Self {
        StyleView {
            id: s.id,
            name: s.name,
            category_id: s.category.id,
            category_name: s.category.name,
            description: s.description,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StyleDetailView {
    pub id: String,
    pub name: String,
    pub category_id: String,
    pub category_name: String,
    pub description: String,
    pub style_json: serde_json::Value,
}

/// Parses `RemoteStyle.style` (mvt-rs stores it as a JSON-encoded string)
/// back into a real JSON value so the model sees nested structure instead
/// of an escaped string blob. Errors only if mvt-rs ever returns a style
/// body that isn't valid JSON, which its own write-path validation should
/// prevent — surfaced rather than silently swallowed if it ever happens.
pub fn build_style_detail_view(style: RemoteStyle) -> Result<StyleDetailView, serde_json::Error> {
    let style_json = serde_json::from_str(&style.style)?;
    Ok(StyleDetailView {
        id: style.id,
        name: style.name,
        category_id: style.category.id,
        category_name: style.category.name,
        description: style.description,
        style_json,
    })
}

/// Builds the model-facing view of one layer, including name+type only for
/// the fields the layer's own `fields: Vec<String>` actually selects (not
/// every column `table_fields` reports) — this minimizes the surface a model
/// could hallucinate a field name against. A layer field name absent from
/// `table_fields` (a stale layer config referencing a dropped column) is
/// silently dropped, not surfaced as an error. Field order is taken from
/// `layer.fields`, not `table_fields`'s column order, since `layer.fields`'s
/// order is the one the layer author actually chose.
pub fn build_layer_view(layer: &RemoteLayer, table_fields: &[RemoteField]) -> LayerView {
    let type_by_name: std::collections::HashMap<&str, &str> = table_fields
        .iter()
        .map(|f| (f.name.as_str(), f.udt.as_str()))
        .collect();

    let fields = layer
        .fields
        .iter()
        .filter_map(|name| {
            type_by_name.get(name.as_str()).map(|udt| FieldView {
                name: name.clone(),
                r#type: udt.to_string(),
            })
        })
        .collect();

    LayerView {
        id: layer.id.clone(),
        name: layer.name.clone(),
        alias: layer.alias.clone(),
        description: layer.description.clone(),
        category_id: layer.category.id.clone(),
        category_name: layer.category.name.clone(),
        geometry: layer.geometry.clone(),
        published: layer.published,
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_only_fields_the_layer_actually_exposes() {
        let category = RemoteCategory {
            id: "cat-1".into(),
            name: "Cities".into(),
            description: "City data".into(),
        };

        let layer = RemoteLayer {
            id: "layer-1".into(),
            category,
            geometry: "Point".into(),
            name: "urban_centers".into(),
            alias: "Urban Centers".into(),
            description: "Major cities".into(),
            database_id: "db-1".into(),
            schema: "public".into(),
            table_name: "cities".into(),
            fields: vec!["name".into(), "population".into()],
            published: true,
        };

        let table_fields = vec![
            RemoteField {
                name: "id".into(),
                udt: "integer".into(),
            },
            RemoteField {
                name: "name".into(),
                udt: "text".into(),
            },
            RemoteField {
                name: "population".into(),
                udt: "bigint".into(),
            },
            RemoteField {
                name: "area".into(),
                udt: "numeric".into(),
            },
            RemoteField {
                name: "created_at".into(),
                udt: "timestamp".into(),
            },
        ];

        let result = build_layer_view(&layer, &table_fields);

        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[0].name, "name");
        assert_eq!(result.fields[0].r#type, "text");
        assert_eq!(result.fields[1].name, "population");
        assert_eq!(result.fields[1].r#type, "bigint");
    }

    #[test]
    fn silently_drops_field_names_not_found_in_the_table() {
        let category = RemoteCategory {
            id: "cat-1".into(),
            name: "Cities".into(),
            description: "City data".into(),
        };

        let layer = RemoteLayer {
            id: "layer-1".into(),
            category,
            geometry: "Point".into(),
            name: "urban_centers".into(),
            alias: "Urban Centers".into(),
            description: "Major cities".into(),
            database_id: "db-1".into(),
            schema: "public".into(),
            table_name: "cities".into(),
            fields: vec!["name".into(), "nonexistent".into(), "population".into()],
            published: true,
        };

        let table_fields = vec![
            RemoteField {
                name: "name".into(),
                udt: "text".into(),
            },
            RemoteField {
                name: "population".into(),
                udt: "bigint".into(),
            },
        ];

        let result = build_layer_view(&layer, &table_fields);

        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[0].name, "name");
        assert_eq!(result.fields[1].name, "population");
    }

    #[test]
    fn preserves_field_order_from_the_layer_not_the_table() {
        let category = RemoteCategory {
            id: "cat-1".into(),
            name: "Cities".into(),
            description: "City data".into(),
        };

        let layer = RemoteLayer {
            id: "layer-1".into(),
            category,
            geometry: "Point".into(),
            name: "urban_centers".into(),
            alias: "Urban Centers".into(),
            description: "Major cities".into(),
            database_id: "db-1".into(),
            schema: "public".into(),
            table_name: "cities".into(),
            fields: vec!["population".into(), "name".into(), "area".into()],
            published: true,
        };

        let table_fields = vec![
            RemoteField {
                name: "area".into(),
                udt: "numeric".into(),
            },
            RemoteField {
                name: "name".into(),
                udt: "text".into(),
            },
            RemoteField {
                name: "population".into(),
                udt: "bigint".into(),
            },
        ];

        let result = build_layer_view(&layer, &table_fields);

        assert_eq!(result.fields.len(), 3);
        assert_eq!(result.fields[0].name, "population");
        assert_eq!(result.fields[1].name, "name");
        assert_eq!(result.fields[2].name, "area");
    }

    #[test]
    fn category_fields_are_flattened_onto_the_layer_view() {
        let category = RemoteCategory {
            id: "cat-123".into(),
            name: "Transportation".into(),
            description: "Roads and transit".into(),
        };

        let layer = RemoteLayer {
            id: "layer-1".into(),
            category,
            geometry: "LineString".into(),
            name: "roads".into(),
            alias: "Roads Network".into(),
            description: "Road network data".into(),
            database_id: "db-1".into(),
            schema: "public".into(),
            table_name: "roads".into(),
            fields: vec!["name".into()],
            published: true,
        };

        let table_fields = vec![RemoteField {
            name: "name".into(),
            udt: "text".into(),
        }];

        let result = build_layer_view(&layer, &table_fields);

        assert_eq!(result.category_id, "cat-123");
        assert_eq!(result.category_name, "Transportation");
    }

    #[test]
    fn style_view_from_remote_style_omits_the_style_body() {
        let remote = RemoteStyle {
            id: "style-1".into(),
            name: "roads-red".into(),
            category: RemoteCategory {
                id: "cat-1".into(),
                name: "Transportation".into(),
                description: "".into(),
            },
            description: "Red road styling".into(),
            style: "{\"version\":8,\"layers\":[]}".into(),
        };

        let view: StyleView = remote.into();

        assert_eq!(view.id, "style-1");
        assert_eq!(view.name, "roads-red");
        assert_eq!(view.category_id, "cat-1");
        assert_eq!(view.category_name, "Transportation");
        assert_eq!(view.description, "Red road styling");
    }

    #[test]
    fn build_style_detail_view_parses_the_style_body_into_json() {
        let remote = RemoteStyle {
            id: "style-1".into(),
            name: "roads-red".into(),
            category: RemoteCategory {
                id: "cat-1".into(),
                name: "Transportation".into(),
                description: "".into(),
            },
            description: "Red road styling".into(),
            style: "{\"version\":8,\"layers\":[]}".into(),
        };

        let view = build_style_detail_view(remote).expect("valid JSON style body must parse");

        assert_eq!(view.id, "style-1");
        assert_eq!(view.category_name, "Transportation");
        assert_eq!(
            view.style_json,
            serde_json::json!({"version": 8, "layers": []})
        );
    }

    #[test]
    fn build_style_detail_view_surfaces_an_unparsable_style_body() {
        let remote = RemoteStyle {
            id: "style-1".into(),
            name: "broken".into(),
            category: RemoteCategory {
                id: "cat-1".into(),
                name: "Transportation".into(),
                description: "".into(),
            },
            description: "".into(),
            style: "not valid json".into(),
        };

        let result = build_style_detail_view(remote);

        assert!(result.is_err());
    }

    #[test]
    fn category_view_from_remote_category() {
        let remote = RemoteCategory {
            id: "cat-42".into(),
            name: "Parks".into(),
            description: "Park areas".into(),
        };

        let view: CategoryView = remote.into();

        assert_eq!(view.id, "cat-42");
        assert_eq!(view.name, "Parks");
        assert_eq!(view.description, "Park areas");
    }
}
