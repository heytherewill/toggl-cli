use crate::error;
use crate::models;
use crate::picker;
use error::PickerError;
use itertools::Itertools;
use models::ResultWithDefaultError;
use picker::{ItemPicker, PickableItem};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct FzfPicker;

fn remove_trailing_newline(value: String) -> String {
    let mut chars = value.chars();
    chars.next_back();
    chars.as_str().to_string()
}

fn format_as_fzf_input<T: PickableItem>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| item.formatted())
        .unique()
        .fold("".to_string(), |acc, item| acc + item.as_str() + "\n")
}

fn create_element_hash_map<T: PickableItem>(items: &[T]) -> HashMap<String, T> {
    items
        .iter()
        .map(|item| (item.formatted(), item.clone()))
        .collect::<HashMap<String, T>>()
}

impl ItemPicker for FzfPicker {

    fn pick<T: PickableItem>(&self, items: Vec<T>) -> ResultWithDefaultError<T> {
        
        let mut command = Command::new("fzf");
        command.arg("-n2..").arg("--ansi").stdin(Stdio::piped()).stdout(Stdio::piped());

        match command.spawn() {
            Ok(mut child) => {

                let fzf_input = format_as_fzf_input(&items);
                let possible_elements = create_element_hash_map(&items);

                writeln!(child.stdin.as_mut().unwrap(), "{}", fzf_input)?;
                
                match child.wait_with_output() {
                    Err(_) => Err(Box::new(PickerError::Generic)),
                    Ok(output) => match output.status.code() {
                        Some(0) => {
                            let user_selected_string = String::from_utf8(output.stdout)?;
                            println!("{}", user_selected_string);
                            let selected_item_index = remove_trailing_newline(user_selected_string);
                            println!("{}", selected_item_index);
                            let selected_item = possible_elements.get(&selected_item_index).unwrap();
                            Ok(selected_item.clone())
                        }

                        Some(128..=254) | None => Err(Box::new(PickerError::Cancelled)),
                        _ => Err(Box::new(PickerError::Generic)),
                    }
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Box::new(PickerError::FzfNotInstalled)),
            Err(_) => Err(Box::new(PickerError::Generic)),
        }
    }
}