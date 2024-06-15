use std::rc::Rc;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ParentCurrency {
    pub factor: f32,
    pub currency: Rc<Currency>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Currency {
    pub name: String,
    pub parent_currency: Option<ParentCurrency>,
}
