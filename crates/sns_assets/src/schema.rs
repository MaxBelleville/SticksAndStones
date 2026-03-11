pub enum SchemaType {
    Primitive,
    Asset(&'static str),
    Optional(Box<SchemaType>),
    Array(Box<SchemaType>),
}

pub struct SchemaField {
    pub name: &'static str,
    pub ty: SchemaType,
}

pub struct Schema {
    pub fields: Vec<SchemaField>,
}
impl Schema {

    pub fn object() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn field(&mut self, field: (&'static str, SchemaType)) {
        self.fields.push(SchemaField {
            name: field.0,
            ty: field.1,
        });
    }

    pub fn primitive() -> SchemaType {
        SchemaType::Primitive
    }

    pub fn asset(name: &'static str) -> SchemaType {
        SchemaType::Asset(name)
    }

    pub fn optional_asset(name: &'static str) -> SchemaType {
        SchemaType::Optional(Box::new(SchemaType::Asset(name)))
    }

    pub fn asset_array(name: &'static str) -> SchemaType {
        SchemaType::Array(Box::new(SchemaType::Asset(name)))
    }

}