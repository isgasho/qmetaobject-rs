use super::*;
use std::rc::{Rc};
use std::cell::{RefCell};

use qmetaobject::scenegraph::{SGNode,ContainerNode,RectangleNode};
use qmetaobject::{QColor, QQuickItem, QRectF};


#[derive(Default)]
pub struct Geometry<'a> {
    pub x: Property<'a,f64>,
    pub y: Property<'a,f64>,
    pub width: Property<'a,f64>,
    pub height: Property<'a,f64>,
}
impl<'a> Geometry<'a> {
    pub fn width(&self) -> f64 { self.width.get() }
    pub fn height(&self) -> f64 { self.height.get() }
    pub fn left(&self) -> f64 { self.x.get() }
    pub fn top(&self) -> f64 { self.y.get() }
    pub fn right(&self) -> f64 { self.x.get() + self.width.get() }
    pub fn bottom(&self) -> f64 { self.y.get() + self.height.get() }
    pub fn vertical_center(&self)  -> f64 { self.x.get() + self.width.get() / 2. }
    pub fn horizontal_center(&self)  -> f64 { self.y.get() + self.height.get() / 2. }
}
/*
enum SizePolicy {
    Fixed(f64),
    Minimum(f64),
    Maximum(f64)
}*/

pub struct LayoutInfo<'a> {
    pub preferred_width : Property<'a, f64>,
    pub preferred_height : Property<'a, f64>,
    pub maximum_width : Property<'a, f64>,
    pub maximum_height : Property<'a, f64>,
    pub minimum_width : Property<'a, f64>,
    pub minimum_height : Property<'a, f64>,
}
impl<'a> Default for LayoutInfo<'a> {
    fn default() -> Self {
        LayoutInfo {
            preferred_width : 0.0.into(),
            preferred_height : 0.0.into(),
            maximum_height : std::f64::MAX.into(),
            maximum_width : std::f64::MAX.into(),
            minimum_width : 0.0.into(),
            minimum_height : 0.0.into(),
        }
    }
}

pub trait Item<'a> {
    fn geometry(&self) -> &Geometry<'a>;
    fn layout_info(&self) -> &LayoutInfo<'a>;
    fn update_paint_node(&self, node : SGNode<ContainerNode>, _item: &QQuickItem)
        -> SGNode<ContainerNode> { node }
}
/*
impl<'a> Item<'a> {
    pub fn new() -> Rc<Self> { Default::default() }
}
*/

pub trait ItemContainer<'a>  {
    fn add_child(&self, child : Rc<Item<'a> + 'a>);
}

/*
#[derive(Default)]
pub struct ItemBase<'a> {
    geometry : Geometry<'a>,
}

impl<'a> GeometryItem<'a> for Item<'a> {
    fn geometry(&self) -> &Geometry<'a> {
        return &self.geometry;
    }
}*/

mod layout_engine {

use std::ops::Add;

pub type Coord = f64;

#[derive(Default)]
pub struct ItemInfo {
    pub min : Coord,
    pub max : Coord,
    pub preferred : Coord,
    pub expand : usize,
}

impl<'a> Add<&'a ItemInfo> for ItemInfo {
    type Output = ItemInfo;

    fn add(self, other: &'a ItemInfo) -> ItemInfo {
        ItemInfo {
            min: self.min + other.min,
            max: self.max + other.max, // the idea is that it saturate with the max value or infinity
            preferred: self.preferred + other.preferred,
            expand: self.expand + other.expand,
        }
    }
}



pub fn compute_total_info(info : &[ItemInfo], spacing : Coord) -> ItemInfo {
    let mut sum : ItemInfo = info.iter().fold(ItemInfo::default(), Add::add);
    let total_spacing = spacing * (info.len() - 1) as Coord;
    sum.min += total_spacing;
    sum.max += total_spacing;
    sum.preferred += total_spacing;
    sum
}

#[derive(Clone, Copy)]
pub struct ItemResult {
    pub size : Coord,
    pub pos : Coord,
}

pub fn do_layout(info : &[ItemInfo], total : ItemInfo, spacing : Coord, size : Coord) -> Vec<ItemResult> {
    // FIXME! consider maximum, or the case where we are smaller that the minimum
    if size < total.preferred {

        let to_remove = total.preferred - size;
        let total_allowed_to_remove = total.preferred - total.min;

        let mut pos = 0 as Coord;
        info.iter().map(|it| {
            let s = it.preferred - (it.preferred - it.min) * to_remove / total_allowed_to_remove;
            let p = pos;
            pos += s + spacing;
            ItemResult { size: s, pos: p }
        }).collect()
    } else {
        let to_add = size - total.preferred;
        //let total_allowed_to_add = total.max - preferred;

        let mut pos = 0 as Coord;
        info.iter().map(|it| {
            let s = if total.expand > 0 {
                it.preferred + to_add * it.expand as Coord / total.expand as Coord
            } else {
                it.preferred + to_add / info.len() as Coord
            };
            let p = pos;
            pos += s + spacing;
            ItemResult { size: s, pos: p }
        }).collect()
    }
}

}

#[derive(Default)]
pub struct ColumnLayout<'a> {
    pub geometry : Geometry<'a>,
    pub layout_info: LayoutInfo<'a>,
    pub spacing: Property<'a, f64>,

    children: RefCell<Vec<Rc<Item<'a> + 'a>>>,
    positions: Property<'a, Vec<layout_engine::ItemResult>>,
}
impl<'a> Item<'a> for ColumnLayout<'a> {
    fn geometry(&self) -> &Geometry<'a> { &self.geometry }
    fn layout_info(&self) -> &LayoutInfo<'a> { &self.layout_info }

    fn update_paint_node(&self, mut node : SGNode<ContainerNode>, item: &QQuickItem) -> SGNode<ContainerNode>
    {
        node.update_dynamic(self.children.borrow().iter(),
            |i, n| i.update_paint_node(n, item) );
        node
    }
}

impl<'a> ItemContainer<'a> for Rc<ColumnLayout<'a>> {
    fn add_child(&self, child : Rc<Item<'a> + 'a>) {
        self.children.borrow_mut().push(child);
        ColumnLayout::build_layout(self);
    }
}

impl<'a> ColumnLayout<'a> {
    pub fn new() -> Rc<Self> { Default::default() }

    fn build_layout(this : &Rc<Self>) {

        // The minimum width is the max of the minimums
        let w = Rc::downgrade(this);
        this.layout_info.minimum_width.set_binding(move || w.upgrade().map_or(0.,|x| {
            x.children.borrow().iter().map(|i| i.layout_info().minimum_width.get())
                .fold(0., f64::max)
        }));

        // The minimum height is the sum of the minimums
        let w = Rc::downgrade(this);
        this.layout_info.minimum_height.set_binding(move || w.upgrade().map_or(0.,|x| {
            x.children.borrow().iter().map(|i| i.layout_info().minimum_height.get())
                .sum()
        }));

        // The maximum width is the min of the maximums
        let w = Rc::downgrade(this);
        this.layout_info.maximum_width.set_binding(move || w.upgrade().map_or(0., |x| {
            x.children.borrow().iter().map(|i| i.layout_info().maximum_width.get())
                .fold(std::f64::MAX, f64::min)
        }));
        // The maximum height is the sum of the maximums (assume it saturates)
        let w = Rc::downgrade(this);
        this.layout_info.maximum_height.set_binding(move || w.upgrade().map_or(0., |x| {
            x.children.borrow().iter().map(|i| i.layout_info().maximum_height.get())
                .sum()
        }));

        // preferred width is the minimum width
        let w = Rc::downgrade(this);
        this.layout_info.preferred_width.set_binding(Some(
            move || Some(w.upgrade()?.layout_info.minimum_width.get())));

        // preferred height is the sum of preferred height
        let w = Rc::downgrade(this);
        this.layout_info.preferred_height.set_binding(move || w.upgrade().map_or(0., |x| {
            x.children.borrow().iter().map(|i| i.layout_info().preferred_height.get())
                .sum()
        }));

        // Set the positions
        let w = Rc::downgrade(this);
        this.positions.set_binding(move || w.upgrade().map_or(Vec::default(), |w|{
            let v = w.children.borrow().iter().map(|x| {
                layout_engine::ItemInfo {
                    min : x.layout_info().minimum_height.get(),
                    max : x.layout_info().maximum_height.get(),
                    preferred : x.layout_info().preferred_height.get(),
                    expand : 1, // FIXME
                }
            }).collect::<Vec<_>>();
            layout_engine::do_layout(&v, layout_engine::compute_total_info(&v, 0.), 0., w.geometry.height())
        }));

        // Set the sizes
        for (idx, x) in this.children.borrow().iter().enumerate() {
            let w = Rc::downgrade(this);
            x.geometry().width.set_binding(Some(move || Some(w.upgrade()?.geometry().width())));
            x.geometry().x.set_binding(|| 0.);
            let w = Rc::downgrade(this);
            x.geometry().height.set_binding(Some(move || Some(w.upgrade()?.positions.get().get(idx)?.size)));
            let w = Rc::downgrade(this);
            x.geometry().y.set_binding(Some(move || Some(w.upgrade()?.positions.get().get(idx)?.pos)));
        }
    }
}
/* DOES NOT WORK  (because we can't get a  &'a Property<'a>)
fn init_fixed_sized<'a>(r: &Item<'a>, width : &'a Property<'a, f64>, height:  &'a Property<'a, f64>) {
    r.layout_info().minimum_height.set_binding(move || height.get());
    r.layout_info().preferred_height.set_binding(move || height.get());
    r.layout_info().maximum_height.set_binding(move || height.get());
    r.layout_info().minimum_width.set_binding(move || width.get());
    r.layout_info().preferred_width.set_binding(move || width.get());
    r.layout_info().maximum_width.set_binding(move || width.get());
} */

#[test]
fn test_layout() {


    #[derive(Default)]
    pub struct LItem<'a> {
        geometry : Geometry<'a>,
        layout_info: LayoutInfo<'a>,
        width: Property<'a, f64>,
        height: Property<'a, f64>,
    }
    impl<'a> Item<'a> for LItem<'a> {
        fn geometry(&self) -> &Geometry<'a> { &self.geometry }
        fn layout_info(&self) -> &LayoutInfo<'a> { &self.layout_info }
    }
    impl<'a> LItem<'a> {
        pub fn new() -> Rc<Self> {
            let r = Rc::new(LItem::default());
            let w = Rc::downgrade(&r);
            r.layout_info.minimum_height.set_binding(move || w.upgrade().map_or(0., |w| w.height.get()));
            let w = Rc::downgrade(&r);
            r.layout_info.preferred_height.set_binding(move || w.upgrade().map_or(0., |w| w.height.get()));
            let w = Rc::downgrade(&r);
            r.layout_info.maximum_height.set_binding(move || w.upgrade().map_or(0., |w| w.height.get()));
            let w = Rc::downgrade(&r);
            r.layout_info.minimum_width.set_binding(move || w.upgrade().map_or(0., |w| w.width.get()));
            let w = Rc::downgrade(&r);
            r.layout_info.preferred_width.set_binding(move || w.upgrade().map_or(0., |w| w.width.get()));
            let w = Rc::downgrade(&r);
            r.layout_info.maximum_width.set_binding(move || w.upgrade().map_or(0., |w| w.width.get()));
            r
        }
    }

    let lay = rsml!{
        ColumnLayout {
            geometry.width: ColumnLayout.layout_info.preferred_width.get(),
            geometry.height: ColumnLayout.layout_info.preferred_height.get(),
        }
    };


    lay.add_child(rsml!{
        LItem {
            width : 150.,
            height : 100.,
        }
    });
    assert_eq!(lay.geometry.width(), 150.);
    assert_eq!(lay.geometry.height(), 100.);
    let middle = rsml!{
        LItem {
            width : 110.,
            height : 90.,
        }
    };
    lay.add_child(middle.clone());
    lay.add_child(rsml!{
        LItem {
            width : 190.,
            height : 60.,
        }
    });
    assert_eq!(lay.geometry.width(), 190.);
    assert_eq!(lay.geometry.height(), 100. + 90. + 60.);

    middle.width.set(200.);
    middle.height.set(50.);

    assert_eq!(lay.geometry.width(), 200.);
    assert_eq!(lay.geometry.height(), 100. + 50. + 60.);

    assert_eq!(lay.geometry.height(), lay.children.borrow()[2].geometry().bottom());

}


#[derive(Default)]
pub struct Rectangle<'a> {
    pub geometry : Geometry<'a>,
    pub layout_info: LayoutInfo<'a>,
    pub color: Property<'a, QColor>,
}

impl<'a> Item<'a> for Rectangle<'a> {
    fn geometry(&self) -> &Geometry<'a> { &self.geometry }
    fn layout_info(&self) -> &LayoutInfo<'a> { &self.layout_info }

    fn update_paint_node(&self, mut node : SGNode<ContainerNode>, item: &QQuickItem) -> SGNode<ContainerNode>
    {
        node.update_static(
            |mut n : SGNode<RectangleNode>| -> SGNode<RectangleNode> {
                n.create(item);
                n.set_color(self.color.get());
                let g = self.geometry();
                n.set_rect(QRectF { x: g.left(), y:g.top(), width: g.width(), height: g.height()  });
                n
            }
        );
        node
    }

}
impl<'a> Rectangle<'a> {
    pub fn new() -> Rc<Self> { Default::default() }
}


/*


/*
rsml! {
    struct Button {
        #geometry,
        text: String,
        clicked: Event
    }
}

rsml! {
    struct Button {
        #geometry,
        text: String,
        clicked: @event
    }
}*/


rsml! {
// #[derive(Item)]
// struct MyWindow {
//    geometry: Geometry,
    ColumnLayout {
//        geometry: MyWindow.geometry
        Button {
           text: "+",
           clicked => { label.text = label.text.parse() + 1;  }
        }
        let label = Label {
           text: "0",
        }
        Button {
           text: "-",
           clicked => { label.text = label.text.parse() - 1;  }
        }
    }
// }
}

*/


use qmetaobject::{QObject};


cpp!{{
#include <QtQuick/QQuickItem>
}}

#[derive(QObject, Default)]
struct QuickItem<'a> {
    base: qt_base_class!(trait QQuickItem),
    node : Option<Rc<Item<'a> + 'a>>,
    init: qt_property!(bool; WRITE set_init)
}
impl<'a> QuickItem<'a> {

    pub fn set_init(&mut self, _: bool) {
        let b : Rc<Rectangle<'a>> =  rsml!( Rectangle { color: QColor::from_name("blue") } );
        let y : Rc<Rectangle<'a>> = rsml!( Rectangle { color: QColor::from_name("yellow") } );

        let i : Rc<ColumnLayout<'a>> = rsml!(
            ColumnLayout {
                geometry.width : 110.,
                geometry.height : 90.,
            }
        );
        i.add_child(b);
        i.add_child(y);
        self.set_node(i);
    }

    pub fn set_node(&mut self, node: Rc<Item<'a> + 'a>) {
        self.node = Some(node);
        let obj = self.get_cpp_object();
        assert!(!obj.is_null());
        cpp!(unsafe [obj as "QQuickItem*"] { obj->setFlag(QQuickItem::ItemHasContents); })
    }
}


impl<'a> QQuickItem for QuickItem<'a>
{
    fn update_paint_node(&mut self, mut node : SGNode<ContainerNode> ) -> SGNode<ContainerNode> {
        if let Some(ref i) = self.node {
            node = i.update_paint_node(node, self);
        }
        node
    }

    fn geometry_changed(&mut self, new_geometry : QRectF, _old_geometry : QRectF) {
        if let Some(ref i) = self.node {
            i.geometry().width.set(new_geometry.width);
            i.geometry().height.set(new_geometry.height);
        }
    }
}
 /*
#[cfg(test)]
mod test {
    #[test]
    fn test() {
        use qmetaobject::*;
        use super::QuickItem;
        qml_register_type::<QuickItem>(cstr!("MyItem"), 1, 0, cstr!("MyItem"));
        let mut engine = QmlEngine::new();
        engine.load_data(r#"
import QtQuick 2.0
import QtQuick.Window 2.0
import MyItem 1.0

Window {
    width: 800
    height: 400
    visible: true

    Rectangle {
        anchors.fill: parent
        anchors.margins: 100
        color: "red"
        border.color: "black"
        border.width: 2
    }

    MyItem {
        anchors.fill: parent
        anchors.margins: 100
        init: true
    }

}



        "#.into());
        engine.exec();
    }

}
  */
