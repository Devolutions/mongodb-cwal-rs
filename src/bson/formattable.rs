/* Copyright 2013 10gen Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use encode::*;
use json_parse::*;
use extra::json;
use std::hashmap::HashMap;

/**
 * Trait for document notations which can be represented as BSON.
 * This trait allows any type to be easily serialized and deserialized as BSON.
 * After implementing this trait on a type Foo, Foo can be converted to
 * a BSON formatted byte representation by calling (Foo::new()).to_bson_t().to_bson();
 */
pub trait BsonFormattable {
    /**
     * Converts an object into a Document.
     * Typically for a struct, an implementation of to_bson_t would convert the struct
     * into a HashMap-based variant of Document (usually Embedded) that would
     * map field names to values.
     */
    fn to_bson_t(&self) -> Document;
    /**
     * Converts a Document into an object of the given type.
     * Logically this method is the inverse of to_bson_t
     * and usually the two functions should roundtrip.
     */
    fn from_bson_t(doc: Document) -> Result<Self,~str>;
}

macro_rules! float_fmt {
    (impl $t:ty) => {
        impl BsonFormattable for $t {
            fn to_bson_t(&self) -> Document {
                (*self as f64).to_bson_t()
            }

            fn from_bson_t(doc: Document) -> Result<$t, ~str> {
                match BsonFormattable::from_bson_t::<f64>(doc) {
                    Ok(i) => Ok(i as $t),
                    Err(e) => Err(e)
                }
            }
        }
    }
}

macro_rules! i32_fmt {
    (impl $t:ty) => {
            impl BsonFormattable for $t {
            fn to_bson_t(&self) -> Document {
                (*self as i32).to_bson_t()
            }

            fn from_bson_t(doc: Document) -> Result<$t, ~str> {
                match BsonFormattable::from_bson_t::<i32>(doc) {
                    Ok(i) => Ok(i as $t),
                    Err(e) => Err(e)
                }
            }
        }
    }
}

float_fmt!{impl f32}
float_fmt!{impl float}
i32_fmt!{impl i8}
i32_fmt!{impl i16}
i32_fmt!{impl int}
i32_fmt!{impl u8}
i32_fmt!{impl u16}
i32_fmt!{impl u32}
i32_fmt!{impl uint}
i32_fmt!{impl char}

impl BsonFormattable for f64 {
    fn to_bson_t(&self) -> Document { Double(*self) }

    fn from_bson_t(doc: Document) -> Result<f64,~str> {
        match doc {
            Double(f) => Ok(f),
            _ => Err(~"can only cast Double to f64")
        }
    }
}

impl BsonFormattable for i32 {
    fn to_bson_t(&self) -> Document { Int32(*self) }

    fn from_bson_t(doc: Document) -> Result<i32,~str> {
        match doc {
            Int32(i) => Ok(i),
            _ => Err(~"can only cast Int32 to i32")
        }
    }
}

impl BsonFormattable for i64 {
    fn to_bson_t(&self) -> Document { Int64(*self) }

    fn from_bson_t(doc: Document) -> Result<i64,~str> {
        match doc {
            Int64(i) => Ok(i),
            UTCDate(i) => Ok(i),
            Timestamp(i) => Ok(i),
            _ => Err(~"can only cast Int64, Date, and Timestamp to i64")
        }
    }
}

impl BsonFormattable for ~str {
    fn to_bson_t(&self) -> Document {
        match ObjParser::from_string::<Document, ExtendedJsonParser<~[char]>>(*self) {
            Ok(doc) => doc,
            Err(e) => fail!("invalid string for parsing: %s", e),
        }
    }

    fn from_bson_t(doc: Document) -> Result<~str,~str> {
        match doc {
            UString(s) => Ok(copy s),
            _ => Err(fmt!("could not convert %? to string", doc))
        }
    }
}

impl<T:BsonFormattable> BsonFormattable for ~T {
    fn to_bson_t(&self) -> Document {
        (**self).to_bson_t()
    }

    fn from_bson_t(doc: Document) -> Result<~T, ~str> {
        match BsonFormattable::from_bson_t(doc) {
            Ok(c) => Ok(~c),
            Err(e) => Err(e)
        }
    }
}

impl BsonFormattable for json::Json {
    fn to_bson_t(&self) -> Document {
        match *self {
            json::Null => Null,
            json::Number(f) => Double(f as f64),
            json::String(ref s) => UString(copy *s),
            json::Boolean(b) => Bool(b),
            json::List(ref l) => l.to_bson_t(),
            json::Object(ref l) => l.to_bson_t(),
        }
    }

    fn from_bson_t(doc: Document) -> Result<json::Json, ~str> {
        match doc {
            Double(f) => Ok(json::Number(f as float)),
            UString(s) => Ok(json::String(copy s)),
            Embedded(a) => Ok(json::Object(~match 
                BsonFormattable::from_bson_t::<HashMap<~str, json::Json>>(Embedded(a)) {
                    Ok(d) => d,
                    Err(e) => return Err(e)    
                })),
            Array(a) => Ok(json::List(match 
                BsonFormattable::from_bson_t::<~[json::Json]>(Embedded(a)) {
                    Ok(d) => d,
                    Err(e) => return Err(e)    
                })),
            Binary(_,_) => Err(~"bindata cannot be translated to Json"),
            ObjectId(_) => Err(~"objid cannot be translated to Json"),
            Bool(b) => Ok(json::Boolean(b)),
            UTCDate(i) => Ok(json::Number(i as float)),
            Null => Ok(json::Null),
            Regex(_,_) => Err(~"regex cannot be translated to Json"),
            JScript(s) => Ok(json::String(copy s)),
            JScriptWithScope(_,_) => Err(~"jscope cannot be translated to Json"),
            Int32(i) => Ok(json::Number(i as float)),
            Timestamp(i) => Ok(json::Number(i as float)),
            Int64(i) => Ok(json::Number(i as float)),
            MinKey => Err(~"minkey cannot be translated to Json"),
            MaxKey => Err(~"maxkey cannot be translated to Json")
        }
    }
}

impl<'self, T:BsonFormattable + Copy> BsonFormattable for ~[T] {
    fn to_bson_t(&self) -> Document {
        let mut doc = BsonDocument::new();
        let s = self.map(|elt| elt.to_bson_t());
        for s.iter().enumerate().advance |(i, &elt)| {
            doc.put(i.to_str(), elt);
        }
        return Array(~doc);
    }

    fn from_bson_t(doc: Document) -> Result<~[T], ~str> {
        match doc {
            Array(d) => {
                let mut ret = ~[];
                for d.fields.iter().advance |&(_,@v)| {
                     match BsonFormattable::from_bson_t::<T>(v) {
                        Ok(elt) => ret.push(elt),
                        Err(e) => return Err(e)
                     }
                }
                return Ok(ret);
            }
            _ => Err(~"only Arrays can be converted to lists")
        }
    }
}

impl<V:BsonFormattable> BsonFormattable for HashMap<~str,V> {
    fn to_bson_t(&self) -> Document {
            let mut doc = BsonDocument::new();
            for self.iter().advance |(&k,&v)| {
                doc.put(k.to_str(),v.to_bson_t());
            }
        return Embedded(~doc);
    }

    fn from_bson_t(doc: Document) -> Result<HashMap<~str,V>, ~str> {
        match doc {
            Embedded(d) => {
                let mut m = HashMap::new();
                for d.fields.iter().advance |&(@k, @v)| {
                    match BsonFormattable::from_bson_t::<V>(v) {
                        Ok(elt) => m.insert(k, elt),
                        Err(e) => return Err(e)
                    };
                }
                return Ok(m);
            }
            Array(d) => {
                let mut m = HashMap::new();
                for d.fields.iter().advance |&(@k, @v)| {
                    match BsonFormattable::from_bson_t::<V>(v) {
                        Ok(elt) => m.insert(k, elt),
                        Err(e) => return Err(e)
                    };
                }
                return Ok(m);
            }
            _ => return Err(~"can only convert Embedded or Array to hashmap")
        }
    }
}

impl BsonFormattable for BsonDocument {
    fn to_bson_t(&self) -> Document {
        Embedded(~(copy *self))
    }

    fn from_bson_t(doc: Document) -> Result<BsonDocument,~str> {
        match doc {
           Embedded(d) => Ok(copy *d),
           Array(d) => Ok(copy *d),
           _ => Err(~"can only convert Embedded and Array to BsonDocument") 
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encode::*;
    use extra::json;

    #[test]
    fn test_json_to_bson() {
        let json = json::List(~[json::Null, json::Number(5f), json::String(~"foo"), json::Boolean(false)]);
        let mut doc = BsonDocument::new();
        doc.put(~"0", Null);
        doc.put(~"1", Double(5f64));
        doc.put(~"2", UString(~"foo"));
        doc.put(~"3", Bool(false));
        assert_eq!(Array(~doc), json.to_bson_t());
    }

    #[test]
    fn test_bson_to_json() {
        assert!(BsonFormattable::from_bson_t::<json::Json>(Double(5.01)).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(UString(~"foo")).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Binary(0u8, ~[0u8])).is_err());
        assert!(BsonFormattable::from_bson_t::<json::Json>(ObjectId(~[0u8])).is_err());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Bool(true)).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(UTCDate(150)).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Null).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Regex(~"A", ~"B")).is_err());
        assert!(BsonFormattable::from_bson_t::<json::Json>(JScript(~"foo")).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Int32(1i32)).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Timestamp(1i64)).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(Int64(1i64)).is_ok());
        assert!(BsonFormattable::from_bson_t::<json::Json>(MinKey).is_err());
        assert!(BsonFormattable::from_bson_t::<json::Json>(MaxKey).is_err());
    }

    #[test]
    fn test_list_to_bson() {
       let l = ~[1,2,3];
       let mut doc = BsonDocument::new();
       doc.put(~"0", Int32(1i32));
       doc.put(~"1", Int32(2i32));
       doc.put(~"2", Int32(3i32));
       assert_eq!(l.to_bson_t(), Array(~doc));
    }

    #[test]
    fn test_bson_to_list() {
       let l = ~[1i32,2,3];
       let mut doc = BsonDocument::new();
       doc.put(~"0", Int32(1i32));
       doc.put(~"1", Int32(2i32));
       doc.put(~"2", Int32(3i32));
       assert_eq!(Ok(l), BsonFormattable::from_bson_t::<~[i32]>(Array(~doc)));
    }
}