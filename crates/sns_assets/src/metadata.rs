pub enum MetaType {
    Primitive,
    Asset(&'static str),
    Optional(Box<MetaType>),
    Array(Box<MetaType>),
}

pub struct MetaField {
    pub name: &'static str,
    pub ty: MetaType,
}

pub struct Meta {
    pub fields: Vec<MetaField>,
}
impl Meta {

    pub fn object() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn field(&mut self, field: (&'static str, MetaType)) {
        self.fields.push(MetaField {
            name: field.0,
            ty: field.1,
        });
    }

    pub fn primitive() -> MetaType {
        MetaType::Primitive
    }

    pub fn asset(name: &'static str) -> MetaType {
        MetaType::Asset(name)
    }

    pub fn optional_asset(name: &'static str) -> MetaType {
        MetaType::Optional(Box::new(MetaType::Asset(name)))
    }

    pub fn asset_array(name: &'static str) -> MetaType {
        MetaType::Array(Box::new(MetaType::Asset(name)))
    }

}