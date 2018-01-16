// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

#[macro_use] extern crate error_chain;

extern crate edn;
extern crate mentat;
extern crate mentat_core;
extern crate rusqlite;
extern crate store;
extern crate time;
extern crate uuid;

use mentat::query::{
    IntoResult,
    QueryExecutionResult,
    Variable,
};
use mentat_core::{
    TypedValue,
    Uuid,
};
use time::Timespec;

pub mod labels;
pub mod items;
pub mod errors;

use errors as list_errors;
pub use labels::Label;
pub use items::{
    Item,
    Items
};
use store::{
    Store,
    StoreConnection,
    ToInner,
    ToTypedValue,
};

#[derive(Debug)]
#[repr(C)]
pub struct Toodle {
    connection: StoreConnection,
}

impl Toodle {
    pub fn new(uri: String) -> Result<Toodle, errors::Error> {
        let store_result = Store::new_store(uri)?;
        let mut toodle = Toodle {
            connection: store_result,
        };

        // TODO proper error handling at the FFI boundary
        toodle.transact_labels_vocabulary().expect("transacted");
        toodle.transact_items_vocabulary().expect("transacted");

        Ok(toodle)
    }
}

fn create_uuid() -> Uuid {
    uuid::Uuid::new_v4()
}

fn return_date_field(results: QueryExecutionResult) -> Result<Option<Timespec>, list_errors::Error> {
    results.into_scalar_result()
        .map(|o| o.and_then(|ts| ts.to_inner()))
        .map_err(|e| e.into())
}

impl Toodle {
    fn item_row_to_item(&self, row: Vec<TypedValue>) -> Item {
        let uuid = row[1].clone().to_inner();
        Item {
            id: row[0].clone().to_inner(),
            uuid: uuid,
            name: row[2].clone().to_inner(),
            due_date: self.fetch_due_date_for_item( &uuid).unwrap_or(None),
            completion_date: self.fetch_completion_date_for_item(&uuid).unwrap_or(None),
            labels: self.fetch_labels_for_item(&uuid).unwrap_or(vec![]),
        }
    }

    pub fn transact_items_vocabulary(&mut self) -> Result<(), list_errors::Error> {
        let schema = r#"[
            {   :db/ident       :item/uuid
                :db/valueType   :db.type/uuid
                :db/cardinality :db.cardinality/one
                :db/unique      :db.unique/value
                :db/index true },
            {   :db/ident       :item/name
                :db/valueType   :db.type/string
                :db/cardinality :db.cardinality/one
                :db/index       true
                :db/fulltext    true  },
            {   :db/ident       :item/due_date
                :db/valueType   :db.type/instant
                :db/cardinality :db.cardinality/one  },
            {   :db/ident       :item/completion_date
                :db/valueType   :db.type/instant
                :db/cardinality :db.cardinality/one  },
            {  :db/ident     :item/label
                :db/valueType :db.type/ref
                :db/cardinality :db.cardinality/many }]"#;
        self.connection
            .transact(schema)
            .map_err(|e| e.into())
            .map(|_| ())
    }

    pub fn transact_labels_vocabulary(&mut self) -> Result<(), list_errors::Error> {
        let schema = r#"[
            {  :db/ident       :label/name
               :db/valueType   :db.type/string
               :db/cardinality :db.cardinality/one
               :db/unique      :db.unique/identity
               :db/index       true
               :db/fulltext    true },
            {  :db/ident       :label/color
               :db/valueType   :db.type/string
               :db/cardinality :db.cardinality/one }]"#;
        self.connection
            .transact(schema)
            .map_err(|e| e.into())
            .map(|_| ())
    }

    pub fn create_label(&mut self, name: String, color: String) -> Result<Option<Label>, list_errors::Error> {
        // TODO: better transact API.
        let query = format!("[{{ :label/name \"{0}\" :label/color \"{1}\" }}]", &name, &color);
        self.connection
            .transact(&query)?;
        self.fetch_label(&name)
    }

    pub fn fetch_label(&self, name: &String) -> Result<Option<Label>, list_errors::Error> {
        let query = r#"[:find [?eid ?name ?color]
                        :in ?name
                        :where
                        [?eid :label/name ?name]
                        [?eid :label/color ?color]
        ]"#;
        self.connection
            .query_args(query, vec![(Variable::from_valid_name("?name"), name.to_typed_value())])
            .into_tuple_result()
            .map(|o| o.as_ref().and_then(Label::from_row))
            .map_err(|e| e.into())
    }

    pub fn fetch_labels(&self) -> Result<Vec<Label>, list_errors::Error> {
        let query = r#"[:find ?eid ?name ?color
                        :where
                        [?eid :label/name ?name]
                        [?eid :label/color ?color]
        ]"#;
        self.connection
            .query(query)
            .into_rel_result()
            .map(|rows| rows.iter().filter_map(|row| Label::from_row(&row)).collect())
            .map_err(|e| e.into())
    }

    pub fn fetch_labels_for_item(&self, item_uuid: &Uuid) -> Result<Vec<Label>, list_errors::Error> {
        let query = r#"[:find ?l ?name ?color
                        :in ?item_uuid
                        :where
                        [?i :item/uuid ?item_uuid]
                        [?i :item/label ?l]
                        [?l :label/name ?name]
                        [?l :label/color ?color]
        ]"#;
        self.connection
            .query_args(query, vec![(Variable::from_valid_name("?item_uuid"), item_uuid.to_typed_value())])
            .into_rel_result()
            .map(|rows| rows.iter().filter_map(|row| Label::from_row(&row)).collect())
            .map_err(|e| e.into())
    }


    pub fn fetch_items_with_label(&self, label: &Label) -> Result<Vec<Item>, list_errors::Error> {
        let query = r#"[:find ?eid ?uuid ?name
                        :in ?label
                        :where
                        [?l :label/name ?label]
                        [?eid :item/label ?l]
                        [?eid :item/uuid ?uuid]
                        [?eid :item/name ?name]
        ]"#;
        self.connection
            .query_args(query, vec![(Variable::from_valid_name("?label"), label.name.to_typed_value())])
            .into_rel_result()
            .map(|rows| rows.into_iter().map(|r| self.item_row_to_item(r)).collect())
            .map_err(|e| e.into())
    }

    pub fn fetch_items(&self) -> Result<Items, list_errors::Error> {
        let query = r#"[:find ?eid ?uuid ?name
                        :where
                        [?eid :item/uuid ?uuid]
                        [?eid :item/name ?name]
        ]"#;

        self.connection
            .query(query)
            .into_rel_result()
            .map(|rows| Items::new(rows.into_iter().map(|r| self.item_row_to_item(r)).collect()))
            .map_err(|e| e.into())
    }

    pub fn fetch_item(&self, uuid: &Uuid) -> Result<Option<Item> , list_errors::Error>{
        let query = r#"[:find [?eid ?uuid ?name]
                        :in ?uuid
                        :where
                        [?eid :item/uuid ?uuid]
                        [?eid :item/name ?name]
        ]"#;
        self.connection
            .query_args(query, vec![(Variable::from_valid_name("?uuid"), uuid.to_typed_value())])
            .into_tuple_result()
            .map(|o| o.map(|r| self.item_row_to_item(r)))
            .map_err(|e| e.into())
    }

    fn fetch_completion_date_for_item(&self, item_id: &Uuid) -> Result<Option<Timespec>, list_errors::Error> {
        let query = r#"[:find ?date .
            :in ?uuid
            :where
            [?eid :item/uuid ?uuid]
            [?eid :item/completion_date ?date]
        ]"#;

        return_date_field(
            self.connection
                .query_args(&query, vec![(Variable::from_valid_name("?uuid"), item_id.to_typed_value())]))
    }

    fn fetch_due_date_for_item(&self, item_id: &Uuid) -> Result<Option<Timespec>, list_errors::Error> {
        let query = r#"[:find ?date .
            :in ?uuid
            :where
            [?eid :item/uuid ?uuid]
            [?eid :item/due_date ?date]
        ]"#;

        let date = return_date_field(
            self.connection
                .query_args(&query, vec![(Variable::from_valid_name("?uuid"), item_id.to_typed_value())]));
        date
    }

    pub fn create_item(&mut self, item: &Item) -> Result<Uuid, list_errors::Error> {
        // TODO: make this mapping better!
        let label_str = item.labels
            .iter()
            .filter(|label| label.id.is_some() )
            .map(|label|  format!("{}", label.id.clone().map::<i64, _>(|e| e.into()).unwrap()) )
            .collect::<Vec<String>>()
            .join(", ");
        let item_uuid = create_uuid();
        let uuid_string = item_uuid.hyphenated().to_string();
        let mut query = format!(r#"[{{
            :item/uuid #uuid {:?}
            :item/name {:?}
            "#, &uuid_string, &(item.name));
        if let Some(due_date) = item.due_date {
            let micro_seconds = due_date.sec * 1000000;
            query = format!(r#"{}:item/due_date #instmicros {}
                "#, &query, &micro_seconds);
        }
        if let Some(completion_date) = item.completion_date {
            let micro_seconds = completion_date.sec * 1000000;
            query = format!(r#"{}:item/completion_date #instmicros {}
                "#, &query, &micro_seconds);
        }
        if !label_str.is_empty() {
            query = format!(r#"{0}:item/label [{1}]
                "#, &query, &label_str);
        }
        query = format!("{0}}}]", &query);
        let _ = self.connection.transact(&query)?;
        Ok(item_uuid)
    }

    pub fn create_and_fetch_item(&mut self, item: &Item) -> Result<Option<Item>, list_errors::Error> {
        let item_uuid = self.create_item(&item)?;
        self.fetch_item(&item_uuid)
    }

    pub fn update_item(&mut self, item: &Item, name: Option<String>, due_date: Option<Timespec>, completion_date: Option<Timespec>, labels: Option<&Vec<Label>>) -> Result<(), list_errors::Error> {
        let item_id = item.id.to_owned().expect("item must have ID to be updated");
        let mut transaction = vec![];

        if let Some(name) = name {
            if item.name != name {
                transaction.push(format!("[:db/add {0} :item/name \"{1}\"]", &item_id.id, name));
            }
        }
        if item.due_date != due_date {
            if let Some(date) = due_date {
                let micro_seconds = date.sec * 1000000;
                transaction.push(format!("[:db/add {:?} :item/due_date #instmicros {}]", &item_id.id, &micro_seconds));
            } else {
                let micro_seconds = item.due_date.unwrap().sec * 1000000;
                transaction.push(format!("[:db/retract {:?} :item/due_date #instmicros {}]", &item_id.id, &micro_seconds));
            }
        }

        if item.completion_date != completion_date {
            if let Some(date) = completion_date {
                let micro_seconds = date.sec * 1000000;
                transaction.push(format!("[:db/add {:?} :item/completion_date #instmicros {}]", &item_id.id, &micro_seconds));
            } else {
                let micro_seconds = item.completion_date.unwrap().sec * 1000000;
                transaction.push(format!("[:db/retract {:?} :item/completion_date #instmicros {}]", &item_id.id, &micro_seconds));
            }
        }

        if let Some(new_labels) = labels {
            let existing_labels = self.fetch_labels_for_item(&(item.uuid)).unwrap_or(vec![]);

            let labels_to_add = new_labels.iter()
                .filter(|label| !existing_labels.contains(label) && label.id.is_some() )
                .map(|label|  format!("{}", label.id.clone().map::<i64, _>(|e| e.into()).unwrap()) )
                .collect::<Vec<String>>()
                .join(", ");
            if !labels_to_add.is_empty() {
                transaction.push(format!("[:db/add {0} :item/label [{1}]]", &item_id.id, labels_to_add));
            }
            let labels_to_remove = existing_labels.iter()
                .filter(|label| !new_labels.contains(label) && label.id.is_some() )
                .map(|label|  format!("{}", label.id.clone().map::<i64, _>(|e| e.into()).unwrap()) )
                .collect::<Vec<String>>()
                .join(", ");
            if !labels_to_remove.is_empty() {
                transaction.push(format!("[:db/retract {0} :item/label [{1}]]", &item_id.id, labels_to_remove));
            }
        }

        // TODO: better transact API.
        let query = format!("[{0}]", transaction.join(""));
        self.connection
            .transact( &query)
            .map(|_| ())
            .map_err(|e| e.into())
    }
}


#[cfg(test)]
mod test {
    extern crate edn;

    use super::{
        Store,
        StoreConnection,
        Toodle,
        Label,
        Item,
        create_uuid,
    };

    use std::sync::Arc;

    use mentat_core::Uuid;
    use time::now_utc;

    fn toodle() -> Toodle {
        Toodle::new(String::new()).expect("Expected a Toodle")
    }

    fn assert_ident_present(edn: edn::Value, namespace: &str, name: &str) -> bool {
        match edn {
            edn::Value::Vector(v) => {
                let mut found = false;
                for val in v.iter() {
                    found = assert_ident_present(val.clone(), namespace, name);
                    if found {
                        break;
                    }
                }
                found
            },
            edn::Value::Map(m) => {
                let mut found = false;
                for (key, val) in &m {
                    if let edn::Value::NamespacedKeyword(ref kw) = *key {
                        if kw.namespace == "db" && kw.name == "ident" {
                            found = assert_ident_present(val.clone(), namespace, name);
                            if found { break; }
                        } else {
                            continue
                        }
                    }
                }
                found
            },
            edn::Value::NamespacedKeyword(kw) => kw.namespace == namespace && kw.name == name,
            _ => false
        }
    }

    #[test]
    fn test_new_toodle() {
        let manager = toodle();
        let schema = manager.connection.fetch_schema();
        assert_ident_present(schema.clone(), "label", "name");
        assert_ident_present(schema, "list", "name");
    }

    #[test]
    fn test_create_label() {
        let mut manager = toodle();
        let name = "test".to_string();
        let color = "#000000".to_string();

        let label = manager.create_label(name.clone(), color.clone()).expect("expected a label option");
        assert!(label.is_some());
        let label = label.unwrap();
        assert!(label.id.is_some());
        assert_eq!(label.name, name);
        assert_eq!(label.color, color);
    }

    #[test]
    fn test_fetch_label() {
        let mut manager = toodle();
        let created_label = manager.create_label("test".to_string(), "#000000".to_string()).expect("expected a label option").expect("Expected a label");
        let fetched_label = manager.fetch_label(&created_label.name).expect("expected a label option").expect("expected a label");
        assert_eq!(fetched_label, created_label);

        let fetched_label = manager.fetch_label(&"doesn't exist".to_string()).expect("expected a label option");
        assert_eq!(fetched_label, None);
    }

    #[test]
    fn test_fetch_labels() {
        let mut manager = toodle();

        let labels = ["label1".to_string(), "label2".to_string(), "label3".to_string()];
        for label in labels.iter() {
            let _  = manager.create_label(label.clone(), "#000000".to_string()).expect("expected a label option");
        }
        let fetched_labels = manager.fetch_labels().expect("expected a vector of labels");
        assert_eq!(fetched_labels.len(), labels.len());
        for label in fetched_labels.iter() {
            assert!(labels.contains(&label.name));
        }
    }

    #[test]
    fn test_create_item() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let date = now_utc().to_timespec();
        let i = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item".to_string(),
            due_date: Some(date.clone()),
            completion_date: Some(date.clone()),
            labels: vec![label, label2]
        };

        let item = manager.create_and_fetch_item(&i).expect("expected an item option").expect("expected an item");
        assert!(!item.uuid.is_nil());
        assert_eq!(item.name, i.name);
        let due_date = item.due_date.expect("expecting a due date");
        assert_eq!(due_date.sec, date.sec);
        let completion_date = item.completion_date.expect("expecting a completion date");
        assert_eq!(completion_date.sec, date.sec);
        assert_eq!(item.labels, i.labels);
    }

    #[test]
    fn test_create_item_no_due_date() {
        let mut manager = toodle();
        let l = Label {
            id: None,
            name: "label1".to_string(),
            color: "#000000".to_string()
        };
        let label = manager.create_label(l.name.clone(), l.color.clone()).expect("expected a label option").unwrap();

        let l2 = Label {
            id: None,
            name: "label2".to_string(),
            color: "#000000".to_string()
        };
        let label2 = manager.create_label(l2.name.clone(), l2.color.clone()).expect("expected an item option").unwrap();

        let date = now_utc().to_timespec();
        let i = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item".to_string(),
            due_date: None,
            completion_date: Some(date.clone()),
            labels: vec![label, label2]
        };

        let item = manager.create_and_fetch_item(&i).expect("expected an item option").expect("expected an item");
        assert!(!item.uuid.is_nil());
        assert_eq!(item.name, i.name);
        assert_eq!(item.due_date, i.due_date);
        let completion_date = item.completion_date.expect("expecting a completion date");
        assert_eq!(completion_date.sec, date.sec);
        assert_eq!(item.labels, i.labels);
    }

    #[test]
    fn test_create_item_no_completion_date() {
        let mut manager = toodle();
        let l = Label {
            id: None,
            name: "label1".to_string(),
            color: "#000000".to_string()
        };
        let label = manager.create_label(l.name.clone(), l.color.clone()).expect("expected a label option").unwrap();

        let l2 = Label {
            id: None,
            name: "label2".to_string(),
            color: "#000000".to_string()
        };
        let label2 = manager.create_label(l2.name.clone(), l2.color.clone()).expect("expected a label option").unwrap();

        let date = now_utc().to_timespec();
        let i = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item".to_string(),
            due_date: Some(date.clone()),
            completion_date: None,
            labels: vec![label, label2]
        };

        let item = manager.create_and_fetch_item(&i).expect("expected an item option").expect("expected an item");
        assert!(!item.uuid.is_nil());
        assert_eq!(item.name, i.name);
        let due_date = item.due_date.expect("expecting a due date");
        assert_eq!(due_date.sec, date.sec);
        assert_eq!(item.completion_date, i.completion_date);
        assert_eq!(item.labels, i.labels);
    }

    #[test]
    fn test_fetch_item() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let mut created_item = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label]
        };

        created_item.uuid = manager.create_item(&created_item).expect("expected a uuid");
        let fetched_item = manager.fetch_item(&created_item.uuid).expect("expected an item option").expect("expected an item");
        assert_eq!(fetched_item.uuid, created_item.uuid);
        assert_eq!(fetched_item.name, created_item.name);
        assert_eq!(fetched_item.due_date, created_item.due_date);
        assert_eq!(fetched_item.completion_date, created_item.completion_date);
        assert_eq!(fetched_item.labels, created_item.labels);

        let tmp_uuid = create_uuid().hyphenated().to_string();
        let item_uuid = Uuid::parse_str(&tmp_uuid).unwrap();
        let fetched_item = manager.fetch_item(&item_uuid).expect("expected an item option");
        assert_eq!(fetched_item, None);
    }

    #[test]
    fn test_fetch_labels_for_item() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label3 = manager.create_label("label3".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let mut item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label, label2, label3]
        };

        item1.uuid = manager.create_item(&item1).expect("expected a uuid");

        let fetched_labels = manager.fetch_labels_for_item(&item1.uuid).expect("expected a vector of labels");
        assert_eq!(fetched_labels, item1.labels);
    }

    #[test]
    fn test_fetch_items_with_label() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label.clone()]
        };
        let item2 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 2".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label.clone()]
        };
        let item3 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 3".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label.clone(), label2.clone()]
        };

        let item4 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 4".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label2.clone()]
        };

        let item1 = manager.create_and_fetch_item(&item1).expect("expected an item option").expect("expected item1");
        let item2 = manager.create_and_fetch_item(&item2).expect("expected an item option").expect("expected item2");
        let item3 = manager.create_and_fetch_item(&item3).expect("expected an item option").expect("expected item3");
        let item4 = manager.create_and_fetch_item(&item4).expect("expected an item option").expect("expected item4");

        let fetched_label1_items = manager.fetch_items_with_label(&label).expect("expected a vector of items");
        assert_eq!(fetched_label1_items, vec![item1, item2, item3.clone()]);
        let fetched_label2_items = manager.fetch_items_with_label(&label2).expect("expected a vector of items");
        assert_eq!(fetched_label2_items, vec![item3, item4]);
    }

    #[test]
    fn test_update_item_add_label() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a labeloption").unwrap();
        let label3 = manager.create_label("label3".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label, label2]
        };

        let mut created_item = manager.create_and_fetch_item(&item1).expect("expected an item option").expect("expected an item");
        let mut new_labels = item1.labels.clone();
        new_labels.push(label3);

        match manager.update_item(&created_item, None, None, None, Some(&new_labels)) {
            Ok(()) => (),
            Err(e) => {
                println!("e {:?}", e);
                assert!(false)
            }
        }

        created_item.labels = new_labels;

        let fetched_item = manager.fetch_item(&created_item.uuid).expect("expected an item option").expect("expected an item");
        assert_eq!(fetched_item, created_item);
    }

    #[test]
    fn test_update_item_remove_label() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label3 = manager.create_label("label3".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label, label2, label3]
        };

        let mut created_item = manager.create_and_fetch_item(&item1).expect("expected an item option").expect("expected an item");
        let mut new_labels = created_item.labels.clone();
        new_labels.remove(2);

        match manager.update_item(&created_item, None, None, None, Some(&new_labels)) {
            Ok(()) => (),
            Err(e) => {
                println!("e {:?}", e);
                assert!(false)
            }
        }

        created_item.labels = new_labels;

        let fetched_item = manager.fetch_item(&created_item.uuid).expect("expected an item option").expect("expected an item");
        assert_eq!(fetched_item, created_item);
    }

    #[test]
    fn test_update_item_add_due_date() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected alabel option").unwrap();
        let label3 = manager.create_label("label3".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let date = now_utc().to_timespec();
        let item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label, label2, label3]
        };

        let created_item = manager.create_and_fetch_item(&item1).expect("expected an item option").expect("expected an item");

        match manager.update_item(&created_item, None, Some(date), None, None) {
            Ok(()) => (),
            Err(e) => {
                println!("e {:?}", e);
                assert!(false)
            }
        }

        let fetched_item = manager.fetch_item(&created_item.uuid).expect("expected an item option").expect("expected an item");
        let due_date = fetched_item.due_date.expect("expected a due date");
        assert_eq!(due_date.sec, date.sec);
    }

    #[test]
    fn test_update_item_change_name() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label3 = manager.create_label("label3".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let date = now_utc().to_timespec();
        let item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: Some(date),
            completion_date: None,
            labels: vec![label, label2, label3]
        };

        let mut created_item = manager.create_and_fetch_item(&item1).expect("expected an item option").expect("expected an item");
        match manager.update_item(&created_item, Some("new name".to_string()), None, None, None) {
            Ok(()) => (),
            Err(e) => {
                println!("e {:?}", e);
                assert!(false)
            }
        }

        created_item.name = "new name".to_string();

        let fetched_item = manager.fetch_item(&created_item.uuid).expect("expected an item option").expect("expected an item");
        assert_eq!(fetched_item.name, created_item.name);
    }

    #[test]
    fn test_update_item_complete_item() {
        let mut manager = toodle();
        let label = manager.create_label("label1".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label2 = manager.create_label("label2".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();
        let label3 = manager.create_label("label3".to_string(), "#000000".to_string()).expect("expected a label option").unwrap();

        let date = now_utc().to_timespec();
        let item1 = Item {
            id: None,
            uuid: Uuid::nil(),
            name: "test item 1".to_string(),
            due_date: None,
            completion_date: None,
            labels: vec![label, label2, label3]
        };

        let created_item = manager.create_and_fetch_item(&item1).expect("expected an item option").expect("expected an item");
        match manager.update_item(&created_item, None, None, Some(date), None) {
            Ok(()) => (),
            Err(e) => {
                println!("e {:?}", e);
                assert!(false)
            }
        }

        let fetched_item = manager.fetch_item(&created_item.uuid).expect("expected an item option").expect("expected an item");
        let completion_date = fetched_item.completion_date.expect("expected a completion_date");
        assert_eq!(completion_date.sec, date.sec);
    }
}