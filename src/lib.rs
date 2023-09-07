// Copyright (c) 2023 Samir Bioud
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
// OR OTHER DEALINGS IN THE SOFTWARE.
// 


use proc_macro::TokenStream;
use quote::quote;

use std::collections::HashMap;

struct AttrArg(HashMap<String, String>);

impl syn::parse::Parse for AttrArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut m = HashMap::<String, String>::new();

        loop {
            if input.is_empty() {
                break;
            }
            let map_from: syn::Ident = input.parse()?;
            let eq: syn::token::Eq = input.parse()?;
            let map_to: syn::Ident = input.parse()?;

            m.insert(map_from.to_string(), map_to.to_string());

            if input.peek(syn::token::Comma) {
                input.parse::<syn::token::Comma>()?;
            } else {
                break;
            }
        }

        return Ok(Self(m));
    }
}

enum Modifier {
    Hide(syn::Ident),
    Show(syn::Ident),
    Optional(syn::Ident),
}
impl Modifier {
    pub fn name(&self) -> String {
        match self {
            Modifier::Hide(i) => i.to_string(),
            Modifier::Show(i) => i.to_string(),
            Modifier::Optional(i) => i.to_string(),
        }
    }
    pub fn name_id(&self) -> syn::Ident {
        match self {
            Modifier::Hide(i) => i.clone(),
            Modifier::Show(i) => i.clone(),
            Modifier::Optional(i) => i.clone(),
        }
    }
}
#[derive(Debug)]
struct ModifierInfo {
    has_hide: bool,
    has_show: bool,
    has_option: bool,
}

fn symbol_mods(mods: &Vec<Modifier>, id: String) -> ModifierInfo {
    let mut has_hide = false;
    let mut has_show = false;
    let mut has_option = false;
    mods.iter().for_each(|mo| match mo {
        Modifier::Hide(i) if i.to_string() == id => has_hide = true,
        Modifier::Show(i) if i.to_string() == id => has_show = true,
        Modifier::Optional(i) if i.to_string() == id => has_option = true,
        _ => {}
    });

    ModifierInfo {
        has_option: has_option,
        has_hide: has_hide,
        has_show: has_show,
    }
}

#[derive(PartialEq)]
enum VisDefault {
    Hide,
    Show,
}

struct Entry {
    modifiers: Vec<Modifier>,
    name: syn::Ident,
    typ: syn::Type,
    vis: syn::Visibility,
    default_show : VisDefault
}

fn validate_modifiers(mods: &Vec<Modifier>) -> Result<(), TokenStream> {
    struct F {
        a: ModifierInfo,
        b: syn::Ident,
    }
    let mut h = HashMap::<String, F>::new();

    mods.iter().for_each(|m| {
        let name = m.name();
        let id = m.name_id();
        if h.get_mut(&name).is_none() {
            h.insert(
                name.clone(),
                F {
                    a: ModifierInfo {
                        has_option: false,
                        has_hide: false,
                        has_show: false,
                    },
                    b: id,
                },
            );
        }
        match m {
            Modifier::Hide(_) => h.get_mut(&name).unwrap().a.has_hide = true,
            Modifier::Show(_) => h.get_mut(&name).unwrap().a.has_show = true,
            Modifier::Optional(_) => h.get_mut(&name).unwrap().a.has_option = true,
        };
    });

    let errors: Vec<_> = h
        .iter()
        .filter_map(|(gen_name, mods)| {
            if mods.a.has_show && mods.a.has_hide {
                Some(syn::Error::new(
                    mods.b.span(),
                    "Conflicting declarations of `show` and `hide` for ".to_owned() + gen_name
                ))
            } else if mods.a.has_hide && mods.a.has_option {
                Some(syn::Error::new(
                    mods.b.span(),
                    "Conflicting declarations of 'hide' and 'optional' for ".to_owned() + &gen_name
                ))
            } else {
                None
            }
        })
        .collect();

    if errors.len() != 0 {
        let errors : Vec<proc_macro2::TokenStream> = errors.iter().map(|e| e.to_compile_error().into()).collect();
        return Err(quote!(
        #(#errors)*
          )
        .into());
    }
    Ok(())
}

struct ParseModifiers{
    default_vis : VisDefault,
    mods : Vec<Modifier>
}

fn parse_modifiers(field: &syn::Field) -> Result<ParseModifiers, TokenStream> {
    let mut ret = Vec::<Modifier>::new();

    let mut def_vis = VisDefault::Show;
    let mut has_explicit_vis = false;

    for attr in field.attrs.iter() {
        match &attr.meta {
            syn::Meta::List(l) => {
                let id = l.path.get_ident().unwrap();
                let mut names = Vec::new();
                l.parse_nested_meta(|c| {
                    if c.path.segments.len() != 1 {
                        Err(c.error("Expected single-segment identifier"))
                    } else {
                        names.push(c.path.segments[0].ident.clone());
                        Ok(())
                    }
                })
                .map_err(|e| e.to_compile_error())?;
                match id.to_string().as_str() {
                    "hide" => {
                        if has_explicit_vis && def_vis == VisDefault::Hide {
                            return Err(syn::Error::new(
                                id.span(),
                                "Double visibilty overrides are not allowed",
                            )
                            .to_compile_error()
                            .into());
                        };
                        names.iter().for_each(|n| {
                            ret.push(Modifier::Hide(n.clone()));
                        });
                        def_vis = VisDefault::Show;
                        has_explicit_vis = true;
                    }
                    "show" => {
                        if has_explicit_vis && def_vis == VisDefault::Show {
                            return Err(syn::Error::new(
                                id.span(),
                                "Double visibilty overrides are not allowed",
                            )
                            .to_compile_error()
                            .into());
                        };
                        def_vis = VisDefault::Hide;
                        has_explicit_vis = true;
                        names.iter().for_each(|n| {
                            ret.push(Modifier::Show(n.clone()));
                        });
                    }
                    "optional" => {
                        names.iter().for_each(|n| {
                            ret.push(Modifier::Optional(n.clone()));
                        });
                    }
                    _ => {
                        return Err(syn::Error::new(
                            id.span(),
                            "Unsupported modifier, expected one of: `show`, `hide`, `optional`",
                        )
                        .to_compile_error()
                        .into())
                    }
                }
            }
            _ => {
                return Err(syn::Error::new(
                    attr.pound_token.span,
                    "Expected meta list Foo(a, b, c)",
                )
                .to_compile_error()
                .into())
            }
        }
        // let name = attr.meta.require_name_value().map_err(|e| e.to_compile_error())?;
        // let id = name.path.get_ident().unwrap();
        // match id.to_string().as_str() {
        // }
    }

    return Ok(ParseModifiers{
        mods : ret,
        default_vis : def_vis
    });
}

///
/// Use this macro to initiate a structur context for a struct
///
#[proc_macro_attribute]
pub fn structur(args: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = syn::parse::<syn::ItemStruct>(item).unwrap();
    let mappings = syn::parse::<AttrArg>(args).unwrap().0;
    let names: Vec<_> = mappings.keys().map(|k| k.clone()).collect();


    let res: Result<Vec<_>, TokenStream> = parsed
        .fields
        .iter()
        .map(|field| {
            let name = field.ident.clone().unwrap();
            let ty = &field.ty;
            let vis = &field.vis;
            let mods = parse_modifiers(&field)?;

            let errors: Vec<_> = mods.mods
                .iter()
                .filter_map(|m| {

                    // Undefined reference error
                    let err = match m {
                        Modifier::Hide(n) if !names.contains(&n.to_string()) => Some(n),
                        Modifier::Show(n) if !names.contains(&n.to_string()) => Some(n),
                        Modifier::Optional(n) if !names.contains(&n.to_string()) => Some(n),
                        _ => None,
                    };
                    if let Some(id) = err {
                            return Some(
                                syn::Error::new(
                                    id.span(),
                                    "Undeclared struct ".to_owned() + &id.to_string(),
                                )
                                .to_compile_error(),
                            );
                    }
                    else{
                        return None;
                    }
                })
                .collect();

            if errors.len() != 0 {
                Err(quote!( #(#errors)* ).into())
            } else {
                Ok(Entry {
                    name: name,
                    typ: ty.clone(),
                    vis: vis.clone(),
                    modifiers: mods.mods,
                    default_show : mods.default_vis
                })
            }
        })
        .collect();

    match res {
        Ok(fields) => {
            let impls: Vec<_> = mappings
                .iter()
                .map(|(pseudo_name, generated_name)| {
                    let field_sources: Result<Vec<_>, _> = fields
                        .iter()
                        .filter_map(|field| {
                            if let Some(e) = validate_modifiers(&field.modifiers).err() {
                                return Some(Err(e));
                            }
                            let modifiers = symbol_mods(&field.modifiers, pseudo_name.to_string());
                            let vis = &field.vis;
                            let ty = &field.typ;
                            let name = &field.name;
                            if modifiers.has_hide {
                                None
                            } else if modifiers.has_option {
                                Some(Ok(quote!( #vis #name : Option<#ty>, )))
                            } else if modifiers.has_show {
                                Some(Ok(quote!( #vis #name : #ty, )))
                            }
                            else{
                                match field.default_show{
                                    VisDefault::Hide => None,
                                    VisDefault::Show => {
                                        Some(Ok(quote!(#vis #name : #ty,)))
                                    }
                                }
                            }
                        })
                        .collect();

                    match field_sources {
                        Ok(source) => {
                            let vis = &parsed.vis;
                            let attrs = &parsed.attrs;
                            let gen_name = quote::format_ident!("{}", generated_name);
                            quote!(
                                #(#attrs)*
                                #vis struct #gen_name {
                                    #(#source)*
                                }
                            )
                        }
                        Err(e) => e.into(),
                    }
                })
                .collect();

            quote!(
            #(#impls)*
              )
            .into()
        }
        Err(e) => e,
    }
}
