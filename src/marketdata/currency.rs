use std::rc::Rc;

#[derive(Debug)]
pub struct ParentCurrency {
    pub factor: f32,
    pub currency: Rc<Currency>,
}

#[derive(Debug)]
pub struct Currency {
    pub name: String,
    pub parent_currency: Option<ParentCurrency>,
}
