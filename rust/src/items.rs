// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

pub use edn::{
    DateTime,
    Utc,
};
use mentat_core::{
    Uuid,
};
use time::Timespec;

use labels::Label;
use store::{
    Entity,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Item {
    pub id: Option<Entity>,
    pub uuid: Uuid,
    pub name: String,
    pub due_date: Option<Timespec>,
    pub completion_date: Option<Timespec>,
    pub labels: Vec<Label>,
}

#[derive(Debug)]
pub struct Items {
    pub vec: Vec<Item>
}

impl Items {
    pub fn new(vec: Vec<Item>) -> Items {
        Items {
            vec: vec
        }
    }
}

impl Drop for Item {
    fn drop(&mut self) {
        println!("{:?} is being deallocated", self);
    }
}
