//! A built-in, plain-language explainer for common open-source licenses. This
//! needs no API key, so license guidance is always available; the LLM summary
//! (when configured) can add nuance on top.

use crate::i18n::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Permissive,
    WeakCopyleft,
    Copyleft,
    PublicDomain,
}

impl Kind {
    pub fn label(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Kind::Permissive, Lang::En) => "Permissive",
            (Kind::Permissive, Lang::Ko) => "관대함",
            (Kind::WeakCopyleft, Lang::En) => "Weak copyleft",
            (Kind::WeakCopyleft, Lang::Ko) => "약한 카피레프트",
            (Kind::Copyleft, Lang::En) => "Copyleft",
            (Kind::Copyleft, Lang::Ko) => "카피레프트",
            (Kind::PublicDomain, Lang::En) => "Public domain",
            (Kind::PublicDomain, Lang::Ko) => "퍼블릭 도메인",
        }
    }
}

pub struct LicenseInfo {
    pub name: &'static str,
    pub kind: Kind,
    summary_en: &'static str,
    summary_ko: &'static str,
}

impl LicenseInfo {
    pub fn summary(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.summary_en,
            Lang::Ko => self.summary_ko,
        }
    }
}

/// Look up a built-in explanation for an SPDX id (tolerates `-only`/`-or-later`).
pub fn explain(spdx: &str) -> Option<LicenseInfo> {
    let key = spdx
        .trim()
        .trim_end_matches("-only")
        .trim_end_matches("-or-later");
    let info = match key {
        "MIT" => LicenseInfo {
            name: "MIT License",
            kind: Kind::Permissive,
            summary_en: "Do almost anything — use, modify, sell — as long as you keep the copyright notice. No warranty.",
            summary_ko: "저작권 고지만 유지하면 사용·수정·판매 등 거의 무엇이든 가능합니다. 보증은 없습니다.",
        },
        "Apache-2.0" => LicenseInfo {
            name: "Apache License 2.0",
            kind: Kind::Permissive,
            summary_en: "Permissive like MIT, plus an explicit patent grant. Keep notices and state your changes.",
            summary_ko: "MIT처럼 관대하면서 특허 사용권까지 명시합니다. 고지를 유지하고 변경 사항을 표기해야 합니다.",
        },
        "BSD-3-Clause" => LicenseInfo {
            name: "BSD 3-Clause",
            kind: Kind::Permissive,
            summary_en: "Permissive; keep the notice and don't use the author's name to endorse derivatives.",
            summary_ko: "관대한 라이선스. 고지를 유지하고, 저작자 이름을 파생물 홍보에 쓰지 않아야 합니다.",
        },
        "BSD-2-Clause" => LicenseInfo {
            name: "BSD 2-Clause",
            kind: Kind::Permissive,
            summary_en: "Permissive; just keep the copyright notice and disclaimer.",
            summary_ko: "관대한 라이선스. 저작권 고지와 면책 조항만 유지하면 됩니다.",
        },
        "ISC" => LicenseInfo {
            name: "ISC License",
            kind: Kind::Permissive,
            summary_en: "Functionally equivalent to MIT — permissive, keep the notice.",
            summary_ko: "MIT와 사실상 동일하게 관대합니다. 고지만 유지하면 됩니다.",
        },
        "MPL-2.0" => LicenseInfo {
            name: "Mozilla Public License 2.0",
            kind: Kind::WeakCopyleft,
            summary_en: "File-level copyleft: changes to MPL files must stay open, but you can combine with proprietary code.",
            summary_ko: "파일 단위 카피레프트. MPL 파일의 변경분은 공개해야 하지만, 독점 코드와 결합은 가능합니다.",
        },
        "LGPL-3.0" | "LGPL-2.1" => LicenseInfo {
            name: "GNU Lesser GPL",
            kind: Kind::WeakCopyleft,
            summary_en: "You can link it from proprietary software, but changes to the library itself must stay open.",
            summary_ko: "독점 소프트웨어에서 링크해 쓸 수 있지만, 라이브러리 자체의 변경분은 공개해야 합니다.",
        },
        "GPL-3.0" => LicenseInfo {
            name: "GNU GPL v3",
            kind: Kind::Copyleft,
            summary_en: "Strong copyleft: anything you distribute that includes it must also be open under GPL-3.0.",
            summary_ko: "강한 카피레프트. 이를 포함해 배포하는 결과물도 GPL-3.0으로 공개해야 합니다.",
        },
        "GPL-2.0" => LicenseInfo {
            name: "GNU GPL v2",
            kind: Kind::Copyleft,
            summary_en: "Strong copyleft: derivatives you distribute must be open under GPL-2.0.",
            summary_ko: "강한 카피레프트. 배포하는 파생물은 GPL-2.0으로 공개해야 합니다.",
        },
        "AGPL-3.0" => LicenseInfo {
            name: "GNU Affero GPL v3",
            kind: Kind::Copyleft,
            summary_en: "GPL plus a network clause: even users who only access it over a network must get the source.",
            summary_ko: "GPL에 네트워크 조항이 더해진 형태. 네트워크로 접속만 하는 사용자에게도 소스를 제공해야 합니다.",
        },
        "Unlicense" | "CC0-1.0" | "0BSD" => LicenseInfo {
            name: "Public domain (Unlicense / CC0)",
            kind: Kind::PublicDomain,
            summary_en: "Effectively no restrictions — dedicated to the public domain.",
            summary_ko: "사실상 제한이 없습니다. 퍼블릭 도메인으로 기증된 라이선스입니다.",
        },
        _ => return None,
    };
    Some(info)
}
