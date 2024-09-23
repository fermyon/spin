#![allow(clippy::from_over_into)]

use wasm_encoder::{
    AbstractHeapType, EntityType, ExportKind, GlobalType, HeapType, MemoryType, RefType, TableType,
    TagKind, TagType, ValType,
};

struct IntoHeapType(wasmparser::HeapType);

impl Into<HeapType> for IntoHeapType {
    fn into(self) -> HeapType {
        match self.0 {
            wasmparser::HeapType::Concrete(_) => {
                panic!("user-defined heap types not yet supported")
            }
            wasmparser::HeapType::Abstract { ty, shared } => {
                let ty = match ty {
                    wasmparser::AbstractHeapType::Func => AbstractHeapType::Func,
                    wasmparser::AbstractHeapType::Extern => AbstractHeapType::Extern,
                    wasmparser::AbstractHeapType::Any => AbstractHeapType::Any,
                    wasmparser::AbstractHeapType::None => AbstractHeapType::None,
                    wasmparser::AbstractHeapType::NoExtern => AbstractHeapType::NoExtern,
                    wasmparser::AbstractHeapType::NoFunc => AbstractHeapType::NoFunc,
                    wasmparser::AbstractHeapType::Eq => AbstractHeapType::Eq,
                    wasmparser::AbstractHeapType::Struct => AbstractHeapType::Struct,
                    wasmparser::AbstractHeapType::Array => AbstractHeapType::Array,
                    wasmparser::AbstractHeapType::I31 => AbstractHeapType::I31,
                    wasmparser::AbstractHeapType::Exn => AbstractHeapType::Exn,
                    wasmparser::AbstractHeapType::NoExn => AbstractHeapType::NoExn,
                };
                HeapType::Abstract { shared, ty }
            }
        }
    }
}

struct IntoRefType(wasmparser::RefType);

impl Into<RefType> for IntoRefType {
    fn into(self) -> RefType {
        RefType {
            nullable: self.0.is_nullable(),
            heap_type: IntoHeapType(self.0.heap_type()).into(),
        }
    }
}

struct IntoValType(wasmparser::ValType);

impl Into<ValType> for IntoValType {
    fn into(self) -> ValType {
        match self.0 {
            wasmparser::ValType::I32 => ValType::I32,
            wasmparser::ValType::I64 => ValType::I64,
            wasmparser::ValType::F32 => ValType::F32,
            wasmparser::ValType::F64 => ValType::F64,
            wasmparser::ValType::V128 => ValType::V128,
            wasmparser::ValType::Ref(ty) => ValType::Ref(IntoRefType(ty).into()),
        }
    }
}

struct IntoTagKind(wasmparser::TagKind);

impl Into<TagKind> for IntoTagKind {
    fn into(self) -> TagKind {
        match self.0 {
            wasmparser::TagKind::Exception => TagKind::Exception,
        }
    }
}

pub struct IntoEntityType(pub wasmparser::TypeRef);

impl Into<EntityType> for IntoEntityType {
    fn into(self) -> EntityType {
        match self.0 {
            wasmparser::TypeRef::Func(index) => EntityType::Function(index),
            wasmparser::TypeRef::Table(ty) => EntityType::Table(TableType {
                element_type: IntoRefType(ty.element_type).into(),
                minimum: ty.initial,
                maximum: ty.maximum,
                table64: ty.table64,
                shared: ty.shared,
            }),
            wasmparser::TypeRef::Memory(ty) => EntityType::Memory(MemoryType {
                minimum: ty.initial,
                maximum: ty.maximum,
                memory64: ty.memory64,
                shared: ty.shared,
                page_size_log2: ty.page_size_log2,
            }),
            wasmparser::TypeRef::Global(ty) => EntityType::Global(GlobalType {
                val_type: IntoValType(ty.content_type).into(),
                mutable: ty.mutable,
                shared: ty.shared,
            }),
            wasmparser::TypeRef::Tag(ty) => EntityType::Tag(TagType {
                kind: IntoTagKind(ty.kind).into(),
                func_type_idx: ty.func_type_idx,
            }),
        }
    }
}

pub struct IntoExportKind(pub wasmparser::ExternalKind);

impl Into<ExportKind> for IntoExportKind {
    fn into(self) -> ExportKind {
        match self.0 {
            wasmparser::ExternalKind::Func => ExportKind::Func,
            wasmparser::ExternalKind::Table => ExportKind::Table,
            wasmparser::ExternalKind::Memory => ExportKind::Memory,
            wasmparser::ExternalKind::Global => ExportKind::Global,
            wasmparser::ExternalKind::Tag => ExportKind::Tag,
        }
    }
}
