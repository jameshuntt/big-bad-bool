use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, Data, DeriveInput, Error, Fields, Ident, LitInt, LitStr, Result, Token, Type,
};

struct BitpackArgs {
    storage: Type,
    version: u32,
}

impl Parse for BitpackArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut storage: Option<Type> = None;
        let mut version: u32 = 1;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "storage" {
                storage = Some(input.parse()?);
            } else if key == "version" {
                let lit: LitInt = input.parse()?;
                version = lit.base10_parse::<u32>()?;
            } else {
                return Err(Error::new_spanned(
                    key,
                    "expected `storage = u8|u16|u32|u64|u128|usize` or `version = n`",
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            storage: storage.ok_or_else(|| {
                Error::new(
                    proc_macro2::Span::call_site(),
                    "missing required `storage = ...` bitpack argument",
                )
            })?,
            version,
        })
    }
}

struct LabelPair {
    name: Ident,
    _eq: Token![=],
    value: LitInt,
}

impl Parse for LabelPair {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

struct FlagPair {
    name: Ident,
    _eq: Token![=],
    bit: LitInt,
}

impl Parse for FlagPair {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _eq: input.parse()?,
            bit: input.parse()?,
        })
    }
}

#[derive(Clone)]
struct ParsedLabel {
    name: Ident,
    value: u128,
}

#[derive(Clone)]
struct ParsedFlag {
    name: Ident,
    bit: u32,
}

#[derive(Clone)]
struct ParsedRange {
    min: u128,
    max: u128,
}

impl Parse for ParsedRange {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let min_lit: LitInt = input.parse()?;
        let min = min_lit.base10_parse::<u128>()?;
        if input.peek(Token![..=]) {
            input.parse::<Token![..=]>()?;
            let max_lit: LitInt = input.parse()?;
            let max = max_lit.base10_parse::<u128>()?;
            Ok(Self { min, max })
        } else {
            Ok(Self { min, max: min })
        }
    }
}

#[derive(Clone)]
enum DepValue {
    Label(Ident),
    Literal(u128),
}

#[derive(Clone)]
struct ParsedDeprecated {
    value: DepValue,
    since: LitStr,
    note: LitStr,
}

impl Parse for ParsedDeprecated {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let value = if input.peek(LitInt) {
            let lit: LitInt = input.parse()?;
            DepValue::Literal(lit.base10_parse::<u128>()?)
        } else {
            DepValue::Label(input.parse()?)
        };

        let mut since: Option<LitStr> = None;
        let mut note: Option<LitStr> = None;

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "since" {
                since = Some(input.parse()?);
            } else if key == "note" {
                note = Some(input.parse()?);
            } else {
                return Err(Error::new_spanned(
                    key,
                    "expected `since = ...` or `note = ...`",
                ));
            }
        }

        Ok(Self {
            value,
            since: since
                .ok_or_else(|| Error::new(input.span(), "deprecated requires `since = \"...\"`"))?,
            note: note
                .ok_or_else(|| Error::new(input.span(), "deprecated requires `note = \"...\"`"))?,
        })
    }
}

#[derive(Clone)]
struct ParsedRequires {
    when_label: Ident,
    other_field: Ident,
    other_label: Ident,
}

impl Parse for ParsedRequires {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let when_label: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        let other_field: Ident = input.parse()?;
        input.parse::<Token![.]>()?;
        let other_label: Ident = input.parse()?;
        Ok(Self {
            when_label,
            other_field,
            other_label,
        })
    }
}

#[derive(Clone)]
struct ConflictList {
    items: Vec<Ident>,
}

impl Parse for ConflictList {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let items = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            items: items.into_iter().collect(),
        })
    }
}

#[derive(Clone)]
struct AliasEntry {
    label: Ident,
    aliases: Vec<LitStr>,
}

impl Parse for AliasEntry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let label: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let content;
        bracketed!(content in input);
        let aliases = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
        Ok(Self {
            label,
            aliases: aliases.into_iter().collect(),
        })
    }
}

#[proc_macro_attribute]
pub fn bitpack(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as BitpackArgs);
    let input = parse_macro_input!(item as DeriveInput);

    match expand_bitpack(args, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_bitpack(args: BitpackArgs, input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let name = input.ident;
    let vis = input.vis;
    let storage_ty = args.storage;
    let schema_version = args.version;
    let storage_bits = storage_bits(&storage_ty)?;

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(named) => named.named,
            _ => {
                return Err(Error::new_spanned(
                    name,
                    "`#[bitpack]` requires a struct with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                name,
                "`#[bitpack]` can only be used on structs",
            ))
        }
    };

    let mut offset: u32 = 0;
    let mut next_default_field_id: u16 = 1;
    let mut used_field_ids: Vec<(u16, Ident)> = Vec::new();
    let mut getters = Vec::new();
    let mut setters = Vec::new();
    let mut flag_helpers = Vec::new();
    let mut constructor_params = Vec::new();
    let mut constructor_arg_names = Vec::new();
    let mut constructor_raw_terms = Vec::new();
    let mut try_constructor_checks = Vec::new();
    let mut checked_constructor_checks = Vec::new();
    let mut consts = Vec::new();
    let mut field_specs = Vec::new();
    let mut label_consts = Vec::new();
    let mut label_entries = Vec::new();
    let mut range_entries = Vec::new();
    let mut flag_consts = Vec::new();
    let mut flag_entries = Vec::new();
    let mut alias_entries = Vec::new();
    let mut deprecated_entries = Vec::new();
    let mut relation_entries = Vec::new();
    let mut dispatch_entries = Vec::new();
    let mut debug_fields = Vec::new();

    for field in fields {
        let field_ident = field.ident.expect("named field");
        let field_ty = field.ty;

        validate_unsigned_integer_type(&field_ty)?;

        let width = read_bits_attr(&field.attrs)?;
        if width == 0 {
            return Err(Error::new_spanned(
                &field_ident,
                "`#[bits(0)]` is not valid; width must be at least 1",
            ));
        }

        if width > integer_bits(&field_ty)? {
            return Err(Error::new_spanned(
                &field_ident,
                format!(
                    "field `{}` has width {} but type `{}` cannot represent that many bits",
                    field_ident,
                    width,
                    type_ident(&field_ty)?
                ),
            ));
        }

        if offset + width > storage_bits {
            return Err(Error::new_spanned(
                &field_ident,
                format!(
                    "bit field overflow: field `{}` ends at bit {}, but storage only has {} bits",
                    field_ident,
                    offset + width,
                    storage_bits
                ),
            ));
        }

        let field_id = read_field_id_attr(&field.attrs)?.unwrap_or(next_default_field_id);
        if field_id == 0 {
            return Err(Error::new_spanned(
                &field_ident,
                "`#[field_id(0)]` is not valid; field ids are 1-based",
            ));
        }
        for (used, used_ident) in &used_field_ids {
            if *used == field_id {
                return Err(Error::new_spanned(
                    &field_ident,
                    format!(
                        "duplicate field id {} used by `{}` and `{}`",
                        field_id, used_ident, field_ident
                    ),
                ));
            }
        }
        used_field_ids.push((field_id, field_ident.clone()));
        next_default_field_id = next_default_field_id.saturating_add(1);

        let parsed_labels = read_labels_attrs(&field.attrs)?;
        validate_labels(&field_ident, width, &parsed_labels)?;

        let parsed_flags = read_flags_attrs(&field.attrs)?;
        validate_flags(&field_ident, width, &parsed_flags)?;

        let reserved_ranges = read_range_attrs(&field.attrs, "reserved")?;
        validate_ranges(&field_ident, width, "reserved", &reserved_ranges)?;
        let invalid_ranges = read_range_attrs(&field.attrs, "invalid")?;
        validate_ranges(&field_ident, width, "invalid", &invalid_ranges)?;

        let default_value = read_default_attr(&field.attrs)?;
        if let Some(default_value) = default_value {
            validate_value(&field_ident, width, default_value, "default")?;
        }

        let description = read_describe_attr(&field.attrs)?
            .unwrap_or_else(|| LitStr::new("", proc_macro2::Span::call_site()));
        let dispatch_role = read_dispatch_attr(&field.attrs)?;
        let deprecated = read_deprecated_attrs(&field.attrs)?;
        let aliases = read_aliases_attrs(&field.attrs)?;
        let requires = read_requires_attrs(&field.attrs)?;
        let conflicts = read_conflicts_attrs(&field.attrs)?;

        validate_aliases(&field_ident, &parsed_labels, &aliases)?;
        validate_deprecated(&field_ident, width, &parsed_labels, &deprecated)?;
        validate_requires(&field_ident, &parsed_labels, &requires)?;
        validate_conflicts(&field_ident, &parsed_labels, &conflicts)?;

        let shift = offset;

        let upper_name = field_ident.to_string().to_uppercase();
        let field_name_const = format_ident!("FIELD_{}", upper_name);
        let field_spec_const = format_ident!("FIELD_{}_SPEC", upper_name);
        let width_const = format_ident!("{}_WIDTH", upper_name);
        let shift_const = format_ident!("{}_SHIFT", upper_name);
        let field_id_const = format_ident!("{}_FIELD_ID", upper_name);
        let value_mask_const = format_ident!("{}_VALUE_MASK", upper_name);
        let mask_const = format_ident!("{}_MASK", upper_name);

        let setter_ident = format_ident!("set_{}", field_ident);
        let try_setter_ident = format_ident!("try_set_{}", field_ident);
        let with_ident = format_ident!("with_{}", field_ident);

        let value_mask_expr = storage_mask_expr(width, storage_bits, &storage_ty);
        let value_mask_u128 = u128_mask_expr(width);
        let mask_u128 = shifted_u128_mask_expr(width, shift);
        let default_tokens = match default_value {
            Some(v) => quote! { Some(#v) },
            None => quote! { None },
        };

        consts.push(quote! {
            pub const #field_name_const: &'static str = stringify!(#field_ident);
            pub const #field_id_const: u16 = #field_id;
            pub const #width_const: u32 = #width;
            pub const #shift_const: u32 = #shift;
            pub const #value_mask_const: #storage_ty = #value_mask_expr;
            pub const #mask_const: #storage_ty = Self::#value_mask_const << Self::#shift_const;
            pub const #field_spec_const: ::big_bad_bool::BitFieldSpec = ::big_bad_bool::BitFieldSpec {
                id: Self::#field_id_const,
                name: Self::#field_name_const,
                shift: Self::#shift_const,
                width: Self::#width_const,
                mask: #mask_u128,
                value_mask: #value_mask_u128,
                description: #description,
                default: #default_tokens,
            };
        });

        for label in parsed_labels.iter() {
            let label_upper = label.name.to_string().to_uppercase();
            let label_const = format_ident!("LABEL_{}_{}", upper_name, label_upper);
            let label_value = label.value;
            let label_name = &label.name;

            label_consts.push(quote! {
                pub const #label_const: ::big_bad_bool::ValueLabel = ::big_bad_bool::ValueLabel::named(
                    Self::#field_name_const,
                    #label_value,
                    stringify!(#label_name),
                );
            });

            label_entries.push(quote! { Self::#label_const });
        }

        for range in reserved_ranges.iter() {
            let min = range.min;
            let max = range.max;
            range_entries.push(quote! { ::big_bad_bool::ValueRange::reserved(Self::#field_name_const, #min, #max) });
        }

        for range in invalid_ranges.iter() {
            let min = range.min;
            let max = range.max;
            range_entries.push(
                quote! { ::big_bad_bool::ValueRange::invalid(Self::#field_name_const, #min, #max) },
            );
        }

        for flag in parsed_flags.iter() {
            let flag_upper = flag.name.to_string().to_uppercase();
            let flag_const = format_ident!("FLAG_{}_{}", upper_name, flag_upper);
            let flag_bit_const = format_ident!("FLAG_{}_{}_BIT", upper_name, flag_upper);
            let has_ident = format_ident!("has_{}_{}", field_ident, flag.name);
            let with_flag_ident = format_ident!("with_{}_{}", field_ident, flag.name);
            let without_flag_ident = format_ident!("without_{}_{}", field_ident, flag.name);
            let bit = flag.bit;
            let flag_name = &flag.name;
            flag_consts.push(quote! {
                pub const #flag_bit_const: u32 = #bit;
                pub const #flag_const: u128 = 1u128 << #bit;
            });
            flag_entries.push(quote! { ::big_bad_bool::BitFlag::new(Self::#field_name_const, #bit, stringify!(#flag_name), "") });
            flag_helpers.push(quote! {
                #[inline(always)]
                #[must_use]
                pub fn #has_ident(self) -> bool {
                    ((self.#field_ident() as u128) & Self::#flag_const) != 0
                }

                #[inline(always)]
                #[must_use]
                pub fn #with_flag_ident(mut self) -> Self {
                    let value = ((self.#field_ident() as u128) | Self::#flag_const) as #field_ty;
                    self.#setter_ident(value);
                    self
                }

                #[inline(always)]
                #[must_use]
                pub fn #without_flag_ident(mut self) -> Self {
                    let value = ((self.#field_ident() as u128) & !Self::#flag_const) as #field_ty;
                    self.#setter_ident(value);
                    self
                }
            });
        }

        for alias in aliases.iter() {
            let label = &alias.label;
            for alias_str in &alias.aliases {
                alias_entries.push(quote! { ::big_bad_bool::ValueAlias::new(Self::#field_name_const, stringify!(#label), #alias_str) });
            }
        }

        for dep in deprecated.iter() {
            let (value, label_string) = deprecated_value_tokens(&field_ident, &parsed_labels, dep)?;
            let since = &dep.since;
            let note = &dep.note;
            deprecated_entries.push(quote! { ::big_bad_bool::DeprecatedValue::new(Self::#field_name_const, #value, #label_string, #since, #note) });
        }

        for req in requires.iter() {
            let when = &req.when_label;
            let other_field = &req.other_field;
            let other_label = &req.other_label;
            relation_entries.push(quote! { ::big_bad_bool::FieldRelation::requires(Self::#field_name_const, stringify!(#when), stringify!(#other_field), stringify!(#other_label)) });
        }

        for pair in conflict_pairs(&conflicts) {
            let left = pair.0;
            let right = pair.1;
            relation_entries.push(quote! { ::big_bad_bool::FieldRelation::conflicts(Self::#field_name_const, stringify!(#left), stringify!(#right)) });
        }

        if let Some(role) = dispatch_role {
            dispatch_entries.push(quote! { ::big_bad_bool::DispatchHint::new(Self::#field_name_const, stringify!(#role)) });
        }

        getters.push(quote! {
            #[inline(always)]
            #[must_use]
            pub const fn #field_ident(self) -> #field_ty {
                ((self.raw >> Self::#shift_const) & Self::#value_mask_const) as #field_ty
            }
        });

        setters.push(quote! {
            #[inline(always)]
            pub fn #setter_ident(&mut self, value: #field_ty) {
                debug_assert!(
                    (value as u128) <= (Self::#value_mask_const as u128),
                    concat!("bitpack value overflow for field `", stringify!(#field_ident), "`")
                );

                self.raw =
                    (self.raw & !Self::#mask_const)
                    | (((value as #storage_ty) & Self::#value_mask_const) << Self::#shift_const);
            }

            #[inline(always)]
            pub fn #try_setter_ident(&mut self, value: #field_ty) -> bool {
                if (value as u128) > (Self::#value_mask_const as u128) {
                    return false;
                }

                self.raw =
                    (self.raw & !Self::#mask_const)
                    | (((value as #storage_ty) & Self::#value_mask_const) << Self::#shift_const);

                true
            }

            #[inline(always)]
            #[must_use]
            pub fn #with_ident(mut self, value: #field_ty) -> Self {
                self.#setter_ident(value);
                self
            }
        });

        constructor_params.push(quote! { #field_ident: #field_ty });
        constructor_arg_names.push(quote! { #field_ident });
        constructor_raw_terms.push(quote! { (((#field_ident as #storage_ty) & Self::#value_mask_const) << Self::#shift_const) });
        try_constructor_checks.push(quote! {
            if (#field_ident as u128) > (Self::#value_mask_const as u128) {
                return None;
            }
        });
        checked_constructor_checks.push(quote! {
            if (#field_ident as u128) > (Self::#value_mask_const as u128) {
                return Err(::big_bad_bool::BitpackError::field_overflow(
                    stringify!(#field_ident),
                    #field_ident as u128,
                    Self::#value_mask_const as u128,
                ));
            }
        });
        field_specs.push(quote! { Self::#field_spec_const });
        debug_fields.push(quote! { .field(stringify!(#field_ident), &self.#field_ident()) });
        offset += width;
    }

    let total_bits = offset;
    let raw_mask_expr = storage_mask_expr(total_bits, storage_bits, &storage_ty);
    let raw_mask_u128 = u128_mask_expr(total_bits);

    let expanded = quote! {
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        #vis struct #name {
            raw: #storage_ty,
        }

        impl #name {
            pub const VERSION: u32 = #schema_version;
            pub const STORAGE_BITS: u32 = #storage_bits;
            pub const USED_BITS: u32 = #total_bits;
            pub const RAW_MASK: #storage_ty = #raw_mask_expr;
            pub const RAW_MASK_U128: u128 = #raw_mask_u128;
            pub const ZERO: Self = Self { raw: 0 };

            #(#consts)*
            #(#flag_consts)*

            pub const FIELDS: &'static [::big_bad_bool::BitFieldSpec] = &[#(#field_specs),*];

            #(#label_consts)*
            pub const LABELS: &'static [::big_bad_bool::ValueLabel] = &[#(#label_entries),*];
            pub const RANGES: &'static [::big_bad_bool::ValueRange] = &[#(#range_entries),*];
            pub const FLAGS: &'static [::big_bad_bool::BitFlag] = &[#(#flag_entries),*];
            pub const ALIASES: &'static [::big_bad_bool::ValueAlias] = &[#(#alias_entries),*];
            pub const DEPRECATED: &'static [::big_bad_bool::DeprecatedValue] = &[#(#deprecated_entries),*];
            pub const RELATIONS: &'static [::big_bad_bool::FieldRelation] = &[#(#relation_entries),*];
            pub const DISPATCH_HINTS: &'static [::big_bad_bool::DispatchHint] = &[#(#dispatch_entries),*];

            #[inline(always)]
            #[must_use]
            pub const fn raw(self) -> #storage_ty { self.raw }

            #[inline(always)]
            #[must_use]
            pub const fn from_raw(raw: #storage_ty) -> Self { Self { raw: raw & Self::RAW_MASK } }

            #[inline(always)]
            #[must_use]
            pub const fn from_raw_unchecked(raw: #storage_ty) -> Self { Self { raw } }

            #[inline(always)]
            #[must_use]
            pub const fn is_valid_raw(raw: #storage_ty) -> bool { (raw & !Self::RAW_MASK) == 0 }

            #[inline(always)]
            #[must_use]
            pub fn try_from_raw(raw: #storage_ty) -> Option<Self> {
                if Self::is_valid_raw(raw) { Some(Self { raw }) } else { None }
            }

            #[inline(always)]
            pub fn try_from_raw_checked(raw: #storage_ty) -> Result<Self, ::big_bad_bool::BitpackError> {
                if Self::is_valid_raw(raw) {
                    Ok(Self { raw })
                } else {
                    Err(::big_bad_bool::BitpackError::raw_layout_violation(raw as u128, Self::RAW_MASK_U128))
                }
            }

            #[inline(always)]
            #[must_use]
            pub const fn new(#(#constructor_params),*) -> Self {
                Self { raw: (0 as #storage_ty) #( | #constructor_raw_terms )* }
            }

            #[inline(always)]
            #[must_use]
            pub const fn try_new(#(#constructor_params),*) -> Option<Self> {
                #(#try_constructor_checks)*
                Some(Self::new(#(#constructor_arg_names),*))
            }

            #[inline(always)]
            pub fn new_checked(#(#constructor_params),*) -> Result<Self, ::big_bad_bool::BitpackError> {
                #(#checked_constructor_checks)*
                Ok(Self::new(#(#constructor_arg_names),*))
            }

            #(#getters)*
            #(#setters)*
            #(#flag_helpers)*
        }

        impl ::big_bad_bool::PackedWord for #name {
            type Raw = #storage_ty;
            const STORAGE_BITS: u32 = #name::STORAGE_BITS;
            const USED_BITS: u32 = #name::USED_BITS;
            const FIELDS: &'static [::big_bad_bool::BitFieldSpec] = #name::FIELDS;

            #[inline(always)]
            fn raw_u128(self) -> u128 { self.raw as u128 }

            #[inline(always)]
            fn raw_storage(self) -> Self::Raw { self.raw }

            #[inline(always)]
            fn from_raw_u128_truncating(raw: u128) -> Self { Self::from_raw(raw as #storage_ty) }

            #[inline(always)]
            fn try_from_raw_u128(raw: u128) -> Option<Self> {
                if raw > (#storage_ty::MAX as u128) { return None; }
                Self::try_from_raw(raw as #storage_ty)
            }

            #[inline(always)]
            fn try_from_raw_u128_checked(raw: u128) -> Result<Self, ::big_bad_bool::BitpackError> {
                if raw > (#storage_ty::MAX as u128) {
                    return Err(::big_bad_bool::BitpackError::raw_overflow(raw, #storage_ty::MAX as u128));
                }
                Self::try_from_raw_checked(raw as #storage_ty)
            }
        }

        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!(#name))
                    .field("raw", &self.raw)
                    #(#debug_fields)*
                    .finish()
            }
        }
    };

    Ok(expanded)
}

fn read_labels_attrs(attrs: &[Attribute]) -> Result<Vec<ParsedLabel>> {
    let mut labels = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("labels") {
            let pairs =
                attr.parse_args_with(Punctuated::<LabelPair, Token![,]>::parse_terminated)?;
            for pair in pairs {
                labels.push(ParsedLabel {
                    name: pair.name,
                    value: pair.value.base10_parse::<u128>()?,
                });
            }
        }
    }
    Ok(labels)
}

fn read_flags_attrs(attrs: &[Attribute]) -> Result<Vec<ParsedFlag>> {
    let mut flags = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("flags") {
            let pairs =
                attr.parse_args_with(Punctuated::<FlagPair, Token![,]>::parse_terminated)?;
            for pair in pairs {
                flags.push(ParsedFlag {
                    name: pair.name,
                    bit: pair.bit.base10_parse::<u32>()?,
                });
            }
        }
    }
    Ok(flags)
}

fn read_range_attrs(attrs: &[Attribute], attr_name: &str) -> Result<Vec<ParsedRange>> {
    let mut ranges = Vec::new();
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            let parsed =
                attr.parse_args_with(Punctuated::<ParsedRange, Token![,]>::parse_terminated)?;
            ranges.extend(parsed.into_iter());
        }
    }
    Ok(ranges)
}

fn read_default_attr(attrs: &[Attribute]) -> Result<Option<u128>> {
    let mut out = None;
    for attr in attrs {
        if attr.path().is_ident("default") {
            if out.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "duplicate #[default(...)] attribute",
                ));
            }
            let lit: LitInt = attr.parse_args()?;
            out = Some(lit.base10_parse::<u128>()?);
        }
    }
    Ok(out)
}

fn read_field_id_attr(attrs: &[Attribute]) -> Result<Option<u16>> {
    let mut out = None;
    for attr in attrs {
        if attr.path().is_ident("field_id") {
            if out.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "duplicate #[field_id(...)] attribute",
                ));
            }
            let lit: LitInt = attr.parse_args()?;
            out = Some(lit.base10_parse::<u16>()?);
        }
    }
    Ok(out)
}

fn read_describe_attr(attrs: &[Attribute]) -> Result<Option<LitStr>> {
    let mut out = None;
    for attr in attrs {
        if attr.path().is_ident("describe") {
            if out.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "duplicate #[describe(...)] attribute",
                ));
            }
            out = Some(attr.parse_args::<LitStr>()?);
        }
    }
    Ok(out)
}

fn read_dispatch_attr(attrs: &[Attribute]) -> Result<Option<Ident>> {
    let mut out = None;
    for attr in attrs {
        if attr.path().is_ident("dispatch") {
            if out.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "duplicate #[dispatch(...)] attribute",
                ));
            }
            out = Some(attr.parse_args::<Ident>()?);
        }
    }
    Ok(out)
}

fn read_deprecated_attrs(attrs: &[Attribute]) -> Result<Vec<ParsedDeprecated>> {
    let mut out = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("deprecated") {
            out.push(attr.parse_args::<ParsedDeprecated>()?);
        }
    }
    Ok(out)
}

fn read_requires_attrs(attrs: &[Attribute]) -> Result<Vec<ParsedRequires>> {
    let mut out = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("requires") {
            out.push(attr.parse_args::<ParsedRequires>()?);
        }
    }
    Ok(out)
}

fn read_conflicts_attrs(attrs: &[Attribute]) -> Result<Vec<Ident>> {
    let mut out = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("conflicts") {
            let parsed = attr.parse_args::<ConflictList>()?;
            out.extend(parsed.items);
        }
    }
    Ok(out)
}

fn read_aliases_attrs(attrs: &[Attribute]) -> Result<Vec<AliasEntry>> {
    let mut out = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("aliases") {
            let parsed =
                attr.parse_args_with(Punctuated::<AliasEntry, Token![,]>::parse_terminated)?;
            out.extend(parsed.into_iter());
        }
    }
    Ok(out)
}

fn validate_labels(field_ident: &Ident, width: u32, labels: &[ParsedLabel]) -> Result<()> {
    let max = max_for_width(width);
    for (idx, label) in labels.iter().enumerate() {
        if label.value > max {
            return Err(Error::new_spanned(
                &label.name,
                format!("label `{}` for field `{}` has value {} but field width {} only accepts values up to {}", label.name, field_ident, label.value, width, max),
            ));
        }
        for later in labels.iter().skip(idx + 1) {
            if later.name == label.name {
                return Err(Error::new_spanned(
                    &later.name,
                    format!(
                        "duplicate label name `{}` on field `{}`",
                        later.name, field_ident
                    ),
                ));
            }
            if later.value == label.value {
                return Err(Error::new_spanned(
                    &later.name,
                    format!(
                        "duplicate label value {} on field `{}`",
                        later.value, field_ident
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_flags(field_ident: &Ident, width: u32, flags: &[ParsedFlag]) -> Result<()> {
    for (idx, flag) in flags.iter().enumerate() {
        if flag.bit >= width {
            return Err(Error::new_spanned(
                &flag.name,
                format!(
                    "flag `{}` for field `{}` uses bit {} but field width is {}",
                    flag.name, field_ident, flag.bit, width
                ),
            ));
        }
        for later in flags.iter().skip(idx + 1) {
            if later.name == flag.name {
                return Err(Error::new_spanned(
                    &later.name,
                    format!(
                        "duplicate flag name `{}` on field `{}`",
                        later.name, field_ident
                    ),
                ));
            }
            if later.bit == flag.bit {
                return Err(Error::new_spanned(
                    &later.name,
                    format!(
                        "duplicate flag bit {} on field `{}`",
                        later.bit, field_ident
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_ranges(
    field_ident: &Ident,
    width: u32,
    kind: &str,
    ranges: &[ParsedRange],
) -> Result<()> {
    let max = max_for_width(width);
    for range in ranges {
        if range.min > range.max {
            return Err(Error::new_spanned(
                field_ident,
                format!(
                    "{} range min {} is greater than max {}",
                    kind, range.min, range.max
                ),
            ));
        }
        if range.max > max {
            return Err(Error::new_spanned(
                field_ident,
                format!(
                    "{} range {}..={} for field `{}` exceeds max {}",
                    kind, range.min, range.max, field_ident, max
                ),
            ));
        }
    }
    Ok(())
}

fn validate_value(field_ident: &Ident, width: u32, value: u128, kind: &str) -> Result<()> {
    let max = max_for_width(width);
    if value > max {
        return Err(Error::new_spanned(
            field_ident,
            format!(
                "{} value {} for field `{}` exceeds max {}",
                kind, value, field_ident, max
            ),
        ));
    }
    Ok(())
}

fn validate_aliases(
    field_ident: &Ident,
    labels: &[ParsedLabel],
    aliases: &[AliasEntry],
) -> Result<()> {
    for alias in aliases {
        if !labels.iter().any(|label| label.name == alias.label) {
            return Err(Error::new_spanned(
                &alias.label,
                format!(
                    "alias target `{}` on field `{}` does not match any label",
                    alias.label, field_ident
                ),
            ));
        }
    }
    Ok(())
}

fn validate_deprecated(
    field_ident: &Ident,
    width: u32,
    labels: &[ParsedLabel],
    deprecated: &[ParsedDeprecated],
) -> Result<()> {
    for dep in deprecated {
        match &dep.value {
            DepValue::Literal(value) => validate_value(field_ident, width, *value, "deprecated")?,
            DepValue::Label(label) => {
                if !labels.iter().any(|known| known.name == *label) {
                    return Err(Error::new_spanned(
                        label,
                        format!(
                            "deprecated label `{}` on field `{}` does not match any label",
                            label, field_ident
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_requires(
    field_ident: &Ident,
    labels: &[ParsedLabel],
    requires: &[ParsedRequires],
) -> Result<()> {
    for req in requires {
        if !labels.iter().any(|known| known.name == req.when_label) {
            return Err(Error::new_spanned(
                &req.when_label,
                format!(
                    "requires source label `{}` on field `{}` does not match any label",
                    req.when_label, field_ident
                ),
            ));
        }
    }
    Ok(())
}

fn validate_conflicts(
    field_ident: &Ident,
    labels: &[ParsedLabel],
    conflicts: &[Ident],
) -> Result<()> {
    if conflicts.len() == 1 {
        return Err(Error::new_spanned(
            &conflicts[0],
            "#[conflicts(...)] needs at least two labels",
        ));
    }
    for conflict in conflicts {
        if !labels.iter().any(|known| known.name == *conflict) {
            return Err(Error::new_spanned(
                conflict,
                format!(
                    "conflict label `{}` on field `{}` does not match any label",
                    conflict, field_ident
                ),
            ));
        }
    }
    Ok(())
}

fn deprecated_value_tokens(
    field_ident: &Ident,
    labels: &[ParsedLabel],
    dep: &ParsedDeprecated,
) -> Result<(u128, proc_macro2::TokenStream)> {
    match &dep.value {
        DepValue::Literal(value) => Ok((*value, quote! { "" })),
        DepValue::Label(label) => {
            let Some(found) = labels.iter().find(|known| known.name == *label) else {
                return Err(Error::new_spanned(
                    label,
                    format!(
                        "deprecated label `{}` on field `{}` does not match any label",
                        label, field_ident
                    ),
                ));
            };
            Ok((found.value, quote! { stringify!(#label) }))
        }
    }
}

fn conflict_pairs(conflicts: &[Ident]) -> Vec<(&Ident, &Ident)> {
    let mut out = Vec::new();
    for i in 0..conflicts.len() {
        for j in (i + 1)..conflicts.len() {
            out.push((&conflicts[i], &conflicts[j]));
        }
    }
    out
}

fn read_bits_attr(attrs: &[Attribute]) -> Result<u32> {
    for attr in attrs {
        if attr.path().is_ident("bits") {
            let lit: LitInt = attr.parse_args()?;
            return lit.base10_parse::<u32>();
        }
    }
    Err(Error::new(
        proc_macro2::Span::call_site(),
        "each bitpack field requires `#[bits(n)]`",
    ))
}

fn storage_bits(ty: &Type) -> Result<u32> {
    let ident = type_ident(ty)?;
    match ident.as_str() {
        "u8" => Ok(8),
        "u16" => Ok(16),
        "u32" => Ok(32),
        "u64" => Ok(64),
        "u128" => Ok(128),
        "usize" => Ok(usize::BITS),
        _ => Err(Error::new_spanned(
            ty,
            "unsupported storage type; use u8, u16, u32, u64, u128, or usize",
        )),
    }
}

fn integer_bits(ty: &Type) -> Result<u32> {
    storage_bits(ty)
}
fn validate_unsigned_integer_type(ty: &Type) -> Result<()> {
    storage_bits(ty).map(|_| ())
}

fn type_ident(ty: &Type) -> Result<String> {
    match ty {
        Type::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            Ok(path.path.segments[0].ident.to_string())
        }
        _ => Err(Error::new_spanned(
            ty,
            "expected a plain unsigned integer type",
        )),
    }
}

fn max_for_width(width: u32) -> u128 {
    if width == 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    }
}

fn storage_mask_expr(width: u32, storage_bits: u32, storage_ty: &Type) -> proc_macro2::TokenStream {
    if width == 0 {
        quote! { 0 as #storage_ty }
    } else if width == storage_bits {
        quote! { !0 as #storage_ty }
    } else {
        quote! { (((1 as #storage_ty) << #width) - 1) }
    }
}

fn u128_mask_expr(width: u32) -> proc_macro2::TokenStream {
    if width == 0 {
        quote! { 0u128 }
    } else if width == 128 {
        quote! { u128::MAX }
    } else {
        quote! { ((1u128 << #width) - 1) }
    }
}

fn shifted_u128_mask_expr(width: u32, shift: u32) -> proc_macro2::TokenStream {
    if width == 0 {
        quote! { 0u128 }
    } else if width == 128 {
        quote! { u128::MAX }
    } else {
        quote! { (((1u128 << #width) - 1) << #shift) }
    }
}
