use scraper::{Html, Selector};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn html_to_mobiledoc(html: &str) -> Value {
    let document = Html::parse_document(html);
    let p_selector = Selector::parse("p").unwrap();

    let mut children_blocks = vec![];

    for p in document.select(&p_selector) {
        let mut children_text = vec![];

        for text_node in p.text() {
            children_text.push(json!({
                "detail": 0,
                "format": 0,
                "mode": "normal",
                "style": "",
                "text": text_node,
                "type": "extended-text",
                "version": 1
            }));
        }

        children_blocks.push(json!({
            "children": children_text,
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "type": "paragraph",
            "version": 1
        }));
    }

    // Retorna a estrutura final
    json!({
        "root": {
            "children": children_blocks,
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "type": "root",
            "version": 1
        }
    })
}

pub fn generate_truncated_uuid() -> String {
    let uuid = Uuid::new_v4(); // Gera um UUID v4 aleatório
    let hex = uuid.as_simple().to_string(); // Formato sem hífens
    hex[..24].to_string() // Trunca para 24 caracteres
}
