#![recursion_limit = "1024"]
#![allow(dead_code)]
#![feature(bind_by_move_pattern_guards)]
#![feature(custom_attribute)]
//#![feature(trace_macros)] trace_macros!(true);

#[macro_use] extern crate quote;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate syn;
extern crate heck;

extern crate itertools;

extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_yaml;

mod type_spec;
mod attribute;
mod read;
mod write;
mod parser;

use std::rc::Rc;

use proc_macro2::TokenStream;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

use crate::attribute::Repeat;
use crate::attribute::Attribute;
use crate::type_spec::TypeSpec;
use crate::type_spec::Meta;
use crate::type_spec::Endian;
use crate::parser::parse;

/*
    TODO
    - instances
    - strings
    - encoding
    - write repeats
    - write instances
    - enums
    - flags
    - byte array
    - process
    - bit-sized ints
    - repeat-until
    - _io
*/

#[proc_macro]
pub fn taikai_from_str(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: TokenStream = input.into();
    let code = quote!( taikai_from_str2!(crate::test_macro, #input); );
    code.into()
}

#[proc_macro]
pub fn taikai_from_str2(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use proc_macro2::TokenStream;
    let runtime: TokenStream = syn::parse_str(include_str!("runtime.rs")).unwrap();

    let parser = Punctuated::<syn::Expr, Token![,]>::parse_separated_nonempty;
    let mut args = parser.parse(input).unwrap();

    let yaml = args.pop().unwrap();
    let scope = args.pop().unwrap().into_value();

    let yaml: syn::LitStr = syn::parse2(quote!(#yaml)).unwrap();

    let scope: syn::Path = syn::parse2(quote!(#scope)).unwrap();
    let scope: Vec<_> = scope.segments.iter().map(|s| s.ident.to_string()).collect();

    let (meta, typ) = parse(&scope, &yaml.value());

    let definition = TypeSpec::define(&[Rc::clone(&typ)]);

    let typ = typ.borrow();
    let root = typ.absolute_final_path();
    let precursor_impls = typ.impl_precursor_reads(&[], &None, &meta);
    let final_read = typ.impl_final_read(&[], &None);
    let final_write = typ.impl_final_write(&[], &None, &meta);
    
    let code = quote!(
        #runtime
        
        #definition

        #(#precursor_impls)*
        #final_read
        #final_write

        impl #root {
            pub fn read<'a>(_input: &'a [u8], _meta: &Meta, _ctx: &Context) -> IoResult<'a, Self> {
                Self::read______None(_input, &(), &(), _meta, _ctx)
            }

            pub fn write<T: std::io::Write>(&self, _io: &mut T, _meta: &Meta, _ctx: &Context) -> std::io::Result<()> {
                self.write______None(_io, &(), &(), _meta, _ctx)
            }
        }
    );
    
    println!("{}", code);

    code.into()
}

#[proc_macro]
pub fn test_simple(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use std::collections::HashMap;

    use proc_macro2::TokenStream;
    let runtime: TokenStream = syn::parse_str(include_str!("runtime.rs")).unwrap();

    let meta = Meta {
        endian: Endian::Big,
    };

    let subtyp = TypeSpec::new(vec![quote!(crate), quote!(test_simple), quote!(__subtypes)],
        "bar".into(),
        HashMap::new(),
        vec![Attribute::new("i", "u8", Repeat::NoRepeat, None, vec![])],
        HashMap::new());
    let mut subtypes = HashMap::new();
    subtypes.insert("bar".into(), subtyp);

    let seq = vec![
        Attribute::new("i", "u8", Repeat::NoRepeat, None, vec![]), 
        Attribute::new("baz", "bar", Repeat::NoRepeat, None, vec![]),
        Attribute::new("j", "u8", Repeat::NoRepeat, None, vec![])
    ];

    let typ = TypeSpec::new(vec![quote!(crate), quote!(test_simple)],
        "root".into(),
        subtypes,
        seq,
        HashMap::new());

    let definition = TypeSpec::define(&[Rc::clone(&typ)]);

    let typ = typ.borrow();
    let root = typ.absolute_final_path();
    let precursor_impls = typ.impl_precursor_reads(&[], &None, &meta);
    let final_impl = typ.impl_final_read(&[], &None);
    
    let code = quote!(
        #runtime
        
        #definition

        #(#precursor_impls)*
        #final_impl

        impl #root {
            pub fn read<'a>(_input: &'a [u8], _meta: &Meta, _ctx: &Context) -> IoResult<'a, Self> {
                Self::read______None(_input, &(), &(), _meta, _ctx)
            }
        }
    );
    
    //println!("{}", code);

    code.into()
}

#[proc_macro]
// example from http://doc.kaitai.io/ksy_reference.html#attribute-type
pub fn test_resolve(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use std::collections::HashMap;

    use proc_macro2::TokenStream;
    let runtime: TokenStream = syn::parse_str(include_str!("runtime.rs")).unwrap();

    let meta = Meta {
        endian: Endian::Big,
    };

    let mut subtypes = HashMap::new();
    {
        let subtyp = TypeSpec::new(vec![quote!(crate), quote!(test_resolve), quote!(__subtypes)],
            "header".into(),
            HashMap::new(),
            vec![Attribute::new("i", "u8", Repeat::NoRepeat, None, vec![])],
            HashMap::new());
        subtypes.insert("header".into(), subtyp);

        let subtyp = TypeSpec::new(vec![quote!(crate), quote!(test_resolve), quote!(__subtypes)],
            "body1".into(),
            HashMap::new(),
            vec![Attribute::new("foo", "super.header", Repeat::NoRepeat, None, vec![])],
            HashMap::new());
        subtypes.insert("body1".into(), subtyp);

        let mut subtypes2 = HashMap::new();
        {
            let subtyp = TypeSpec::new(vec![quote!(crate), quote!(test_resolve), quote!(__subtypes)],
                "header".into(),
                HashMap::new(),
                vec![Attribute::new("j", "u8", Repeat::NoRepeat, None, vec![])],
                HashMap::new());
            subtypes2.insert("header".into(), subtyp);
        }
        let subtyp = TypeSpec::new(vec![quote!(crate), quote!(test_resolve), quote!(__subtypes)],
            "body2".into(),
            subtypes2,
            vec![Attribute::new("foo", "header", Repeat::NoRepeat, None, vec![])],
            HashMap::new());
        subtypes.insert("body2".into(), subtyp);
    }

    let seq = vec![
        Attribute::new("foo", "header", Repeat::NoRepeat, None, vec![]),
        Attribute::new("bar", "body1", Repeat::NoRepeat, None, vec![]),
        Attribute::new("baz", "body2", Repeat::NoRepeat, None, vec![])
    ];

    let typ = TypeSpec::new(vec![quote!(crate), quote!(test_resolve)],
        "root".into(),
        subtypes,
        seq,
        HashMap::new());
    let definition = TypeSpec::define(&[Rc::clone(&typ)]);

    let typ = typ.borrow();
    let root = typ.absolute_final_path();
    let precursor_impls = typ.impl_precursor_reads(&[], &None, &meta);
    let final_impl = typ.impl_final_read(&[], &None);
    
    let code = quote!(
        #runtime
        
        #definition

        #(#precursor_impls)*
        #final_impl

        impl #root {
            pub fn read<'a>(_input: &'a [u8], _meta: &Meta, _ctx: &Context) -> IoResult<'a, Self> {
                Self::read______None(_input, &(), &(), _meta, _ctx)
            }
        }
    );
    
    //println!("{}", code);

    code.into()
}

#[proc_macro]
pub fn test_compound(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use std::collections::HashMap;

    use proc_macro2::TokenStream;
    let runtime: TokenStream = syn::parse_str(include_str!("runtime.rs")).unwrap();

    let meta = Meta {
        endian: Endian::Big,
    };

    let subtyp = TypeSpec::new(vec![quote!(crate), quote!(test_compound), quote!(__subtypes)],
        "bar".into(),
        HashMap::new(),
        vec![Attribute::new("i", "u8", Repeat::Expr(quote!(3)), None, vec![])],
            HashMap::new());
    let mut subtypes = HashMap::new();
    subtypes.insert("bar".into(), subtyp);

    let seq = vec![
        Attribute::new("i", "u16be", Repeat::NoRepeat, None, vec![]), 
        Attribute::new("j", "u16le", Repeat::NoRepeat, None, vec![]), 
        Attribute::new("baz", "bar", Repeat::NoRepeat, Some(quote!(self.i == 0x0102)), vec![]),
        Attribute::new("k", "u8le", Repeat::Expr(quote!(1)), Some(quote!(self.i == 0x9999)), vec![]),
        Attribute::new("l", "u8le", Repeat::NoRepeat, Some(quote!(true)), vec![]),
        Attribute::new("bytes", "u8", Repeat::Expr(quote!(2)), Some(quote!(true)), b"abc".to_vec()),
    ];

    let typ = TypeSpec::new(vec![quote!(crate), quote!(test_compound)],
        "root".into(),
        subtypes,
        seq,
        HashMap::new());

    let definition = TypeSpec::define(&[Rc::clone(&typ)]);

    let typ = typ.borrow();
    let root = typ.absolute_final_path();
    let precursor_impls = typ.impl_precursor_reads(&[], &None, &meta);
    let final_impl = typ.impl_final_read(&[], &None);
    
    let code = quote!(
        #runtime
        
        #definition

        #(#precursor_impls)*
        #final_impl

        impl #root {
            pub fn read<'a>(_input: &'a [u8], _meta: &Meta, _ctx: &Context) -> IoResult<'a, Self> {
                Self::read______None(_input, &(), &(), _meta, _ctx)
            }
        }
    );
    
    //println!("{}", code);

    code.into()
}
