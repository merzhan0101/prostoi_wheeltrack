#[derive(Debug, Clone, Copy)]
pub struct AreaConfig {
    pub slug: &'static str,
    pub name: &'static str,
    pub department_id: &'static str,
    pub place_id: i32,
}

pub const AREAS: &[AreaConfig] = &[
    AreaConfig {
        slug: "workpiece",
        name: "Заготовительный участок КПЦ",
        department_id: "92b138d1-c131-11ed-8163-00155d16fe68",
        place_id: 1,
    },
    AreaConfig {
        slug: "rolling",
        name: "Прессопрокатный участок",
        department_id: "92b138d4-c131-11ed-8163-00155d16fe68",
        place_id: 4,
    },
    AreaConfig {
        slug: "utk",
        name: "Участок термообработки колес",
        department_id: "92b138e6-c131-11ed-8163-00155d16fe68",
        place_id: 7,
    },
    AreaConfig {
        slug: "uppo",
        name: "Участок полнопрофильной обработки КПЦ",
        department_id: "92b138cc-c131-11ed-8163-00155d16fe68",
        place_id: 10,
    },
    AreaConfig {
        slug: "unk",
        name: "Участок неразрушающего контроля КПЦ",
        department_id: "92b138e4-c131-11ed-8163-00155d16fe68",
        place_id: 5,
    }
];

pub fn find_area(slug: &str) -> Option<&'static AreaConfig> {
    AREAS.iter().find(|area| area.slug == slug)
}