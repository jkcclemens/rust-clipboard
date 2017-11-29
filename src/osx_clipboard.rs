/*
Copyright 2016 Avraham Weinstock

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use common::*;
use objc::runtime::{Object, Class};
use objc_foundation::{INSData, INSString};
use objc_foundation::{NSArray, NSData, NSString};
use objc_id::{Id};
use std::error::Error;
use std::mem::transmute;

pub struct OSXClipboardContext {
    pasteboard: Id<Object>,
}

// required to bring NSPasteboard into the path of the class-resolver
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

impl ClipboardProvider for OSXClipboardContext {
    fn new() -> Result<OSXClipboardContext, Box<Error>> {
        let cls = try!(Class::get("NSPasteboard").ok_or(err("Class::get(\"NSPasteboard\")")));
        let pasteboard: *mut Object = unsafe { msg_send![cls, generalPasteboard] };
        if pasteboard.is_null() {
            return Err(err("NSPasteboard#generalPasteboard returned null"));
        }
        let pasteboard: Id<Object> = unsafe { Id::from_ptr(pasteboard) };
        Ok(OSXClipboardContext { pasteboard: pasteboard })
    }
    fn get_contents(&mut self) -> Result<(Vec<u8>, String), Box<Error>> {
        let pb_item_array: Id<NSArray<Object>> = unsafe {
            let obj: *mut _ =
                msg_send![self.pasteboard, pasteboardItems];
            if obj.is_null() {
                return Err(err("pasteboard#readObjectsForClasses:options: returned null"));
            }
            Id::from_ptr(obj)
        };
        let count: usize = unsafe { msg_send![pb_item_array, count] };
        if count == 0 {
            return Err(err("pasteboard#readObjectsForClasses:options: returned empty"))
        }
        let pb_item: *const Object = unsafe {
            let obj: *const Object = msg_send![pb_item_array, objectAtIndex:0];
            &*obj
        };
        let types: Id<NSArray<Object>> = unsafe {
            let obj: *mut _ = msg_send![pb_item, types];
            if obj.is_null() {
                return Err(err("pasteboardItem#types: returned null"));
            }
            Id::from_ptr(obj)
        };
        let count: usize = unsafe { msg_send![types, count] };
        if count == 0 {
            return Err(err("pasteboardItem#types: returned empty"));
        }
        let kind = unsafe {
            let obj: *const Object = msg_send![types, objectAtIndex:0];
            &*obj
        };
        let kind_str: &NSString = unsafe { transmute(kind) };
        let data: Id<NSData> = unsafe {
            let obj: *mut _ = msg_send![pb_item, dataForType:kind];
            if obj.is_null() {
                return Err(err("pasteboardItem#dataForType:type: returned null"));
            }
            Id::from_ptr(obj)
        };
        Ok((data.bytes().to_vec(), kind_str.as_str().to_owned()))
    }
    fn set_contents(&mut self, data: Vec<u8>, kind: String) -> Result<(), Box<Error>> {
        let cls = Class::get("NSPasteboardItem").ok_or(err("Class::get(\"NSPasteboardItem\")"))?;
        let item: Id<Object> = unsafe { Id::from_ptr(msg_send![cls, new]) };
        let success: bool = unsafe {
            msg_send![item, setData:NSData::from_vec(data) forType:NSString::from_str(&kind)]
        };
        if !success {
            return Err(err("NSPasteboardItem#setData: returned false"));
        }
        let _: usize = unsafe { msg_send![self.pasteboard, clearContents] };
        let refs = vec![item];
        let item_array: Id<NSArray<Object>> = unsafe {
            let cls = Class::get("NSArray").unwrap();
            let obj: *mut NSArray<Object> = msg_send![cls, alloc];
            let obj: *mut NSArray<Object> = msg_send![obj, initWithObjects:refs.as_ptr()
                                                            count:refs.len()];
            Id::from_retained_ptr(obj)
        };
        let success: bool = unsafe { msg_send![self.pasteboard, writeObjects:item_array] };
        return if success {
            Ok(())
        } else {
            Err(err("NSPasteboard#writeObjects: returned false"))
        };
    }
}

// this is a convenience function that both cocoa-rs and
//  glutin define, which seems to depend on the fact that
//  Option::None has the same representation as a null pointer
#[inline]
pub fn class(name: &str) -> *mut Class {
    unsafe { transmute(Class::get(name)) }
}
