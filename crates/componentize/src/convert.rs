#![allow(clippy::from_over_into)]

use {
    wasm_encoder::{
        EntityType, ExportKind, GlobalType, HeapType, MemoryType, RefType, TableType, TagKind,
        TagType, ValType,
    },
    wasmparser::{ExternalKind, TypeRef},
};

struct IntoHeapType(wasmparser::HeapType);

impl Into<HeapType> for IntoHeapType {
    fn into(self) -> HeapType {
        match self.0 {
            wasmparser::HeapType::Func => HeapType::Func,
            wasmparser::HeapType::Extern => HeapType::Extern,
            wasmparser::HeapType::Concrete(_) => {
                panic!("user-defined heap types not yet supported")
            }
            wasmparser::HeapType::Any => HeapType::Any,
            wasmparser::HeapType::None => HeapType::None,
            wasmparser::HeapType::NoExtern => HeapType::NoExtern,
            wasmparser::HeapType::NoFunc => HeapType::NoFunc,
            wasmparser::HeapType::Eq => HeapType::Eq,
            wasmparser::HeapType::Struct => HeapType::Struct,
            wasmparser::HeapType::Array => HeapType::Array,
            wasmparser::HeapType::I31 => HeapType::I31,
            wasmparser::HeapType::Exn => HeapType::Exn,
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

pub struct IntoEntityType(pub TypeRef);

impl Into<EntityType> for IntoEntityType {
    fn into(self) -> EntityType {
        match self.0 {
            TypeRef::Func(index) => EntityType::Function(index),
            TypeRef::Table(ty) => EntityType::Table(TableType {
                element_type: IntoRefType(ty.element_type).into(),
                minimum: ty.initial,
                maximum: ty.maximum,
            }),
            TypeRef::Memory(ty) => EntityType::Memory(MemoryType {
                minimum: ty.initial,
                maximum: ty.maximum,
                memory64: ty.memory64,
                shared: ty.shared,
            }),
            TypeRef::Global(ty) => EntityType::Global(GlobalType {
                val_type: IntoValType(ty.content_type).into(),
                mutable: ty.mutable,
            }),
            TypeRef::Tag(ty) => EntityType::Tag(TagType {
                kind: IntoTagKind(ty.kind).into(),
                func_type_idx: ty.func_type_idx,
            }),
        }
    }
}

pub struct IntoExportKind(pub ExternalKind);

impl Into<ExportKind> for IntoExportKind {
    fn into(self) -> ExportKind {
        match self.0 {
            ExternalKind::Func => ExportKind::Func,
            ExternalKind::Table => ExportKind::Table,
            ExternalKind::Memory => ExportKind::Memory,
            ExternalKind::Global => ExportKind::Global,
            ExternalKind::Tag => ExportKind::Tag,
        }
    }
}
