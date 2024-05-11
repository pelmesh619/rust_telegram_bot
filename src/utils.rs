use grammers_tl_types::enums::MessageEntity;
use grammers_tl_types::types::{MessageEntityBlockquote, MessageEntityBold, MessageEntityCode, MessageEntityCustomEmoji, MessageEntityItalic, MessageEntityPre, MessageEntitySpoiler, MessageEntityStrike, MessageEntityTextUrl, MessageEntityUnderline};
use regex::Regex;


pub fn parse_entities(text: &str) -> (String, Vec<MessageEntity>) {
    let mut result = Vec::<MessageEntity>::new();

    // deleting whitespaces from begin and end
    let regex_begin = Regex::new("^\\s*(<[\\w<>=\\s\"]*>)\\s*").unwrap();
    let regex_end = Regex::new("\\s*(</[\\w</>]*>)\\s*$").unwrap();

    let text = &*regex_begin.replace(text, "$1");
    let text = &*regex_end.replace(text, "$1");
    let text = text.replace("\n", "\\n"); // kostyl
    let mut new_text = String::new();

    let r = html_parser::Dom::parse(text.as_str()).unwrap();

    fn rec_parse(cur: &html_parser::Node, offset: usize, result: &mut Vec<MessageEntity>, new_text: &mut String) -> usize {
        if let Some(t) = cur.text() {
            let new_t = t.replace("\\n", "\n");
            *new_text += &*new_t;
            return new_t.encode_utf16().collect::<Vec<_>>().len()
        }

        if let Some(e) = cur.element() {
            let mut len = 0;

            for i in 0..e.children.len() {
                len += rec_parse(&e.children[i], offset + len, result, new_text);
            }
            let entity = match e.name.as_str() {
                "i" | "em" => {
                    Some(MessageEntity::Italic(
                        MessageEntityItalic{ offset: offset as i32, length: len as i32 })
                    )
                }
                "b" | "strong" => {
                    Some(MessageEntity::Bold(
                        MessageEntityBold{ offset: offset as i32, length: len as i32 })
                    )
                }
                "u" => {
                    Some(MessageEntity::Underline(
                        MessageEntityUnderline{ offset: offset as i32, length: len as i32 })
                    )
                }
                "s" | "del" | "strike" => {
                    Some(MessageEntity::Strike(
                        MessageEntityStrike{ offset: offset as i32, length: len as i32 })
                    )
                }
                "code" => {
                    Some(MessageEntity::Code(
                        MessageEntityCode{ offset: offset as i32, length: len as i32 })
                    )
                }
                "pre" => {
                    if let Some(option) = e.attributes.get("language") {
                        if let Some(language) = option {
                            Some(MessageEntity::Pre(
                                MessageEntityPre{ offset: offset as i32, length: len as i32, language: language.clone() })
                            )
                        } else { None }
                    } else {
                        Some(MessageEntity::Pre(
                            MessageEntityPre{ offset: offset as i32, length: len as i32, language: "".to_string() })
                        )
                    }
                }
                "a" => {
                    if let Some(option) = e.attributes.get("href") {
                        if let Some(url) = option {
                            Some(MessageEntity::TextUrl(
                                MessageEntityTextUrl{ offset: offset as i32, length: len as i32, url: url.clone() })
                            )
                        } else { None }
                    } else { None }
                }
                "emoji" => {
                    if let Some(option) = e.attributes.get("document_id") {
                        if let Some(document_id) = option {
                            Some(MessageEntity::CustomEmoji(
                                MessageEntityCustomEmoji { offset: offset as i32, length: len as i32, document_id: document_id.parse::<i64>().unwrap_or(0) })
                            )
                        } else { None }
                    } else if let Some(document_id) = &e.id {
                        Some(MessageEntity::CustomEmoji(
                            MessageEntityCustomEmoji { offset: offset as i32, length: len as i32, document_id: document_id.parse::<i64>().unwrap_or(0) })
                        )
                    } else { None }
                }
                "blockquote" => {
                    Some(MessageEntity::Blockquote(
                        MessageEntityBlockquote{ offset: offset as i32, length: len as i32 })
                    )
                }
                "spoiler" => {
                    Some(MessageEntity::Spoiler(
                        MessageEntitySpoiler{ offset: offset as i32, length: len as i32 })
                    )
                }
                _ => { None }
            };
            if let Some(entity) = entity {
                if entity.length() > 0 {
                    result.push(entity);
                }
            }
            len
        } else {
            0
        }
    }

    let mut offset = 0usize;

    for i in r.children {
        offset += rec_parse(&i, offset, &mut result, &mut new_text);
    }

    (new_text.parse().unwrap(), result)
}