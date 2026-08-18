//! 中文字体注册(参考 etest data/font.rs)
//!
//! egui 默认字体不含 CJK,国产 Linux 缺字体时会显示方框;
//! 这里从常见系统路径加载中文字体放 Proportional/Monospace 首位,
//! 首个命中即停:Windows 微软雅黑/黑体 -> Kylin/UOS/Ubuntu Noto CJK/文泉驿。
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

const CJK_FONT: &str = "cjk";

pub fn load(ctx: &egui::Context) {
  let mut fonts = FontDefinitions::default();
  let candidates = [
    // Windows 10/11
    "C:/Windows/Fonts/msyh.ttc",
    "C:/Windows/Fonts/simhei.ttf",
    // 银河麒麟 V10 SP1
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    // 统信 UOS V20
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/xfonts-wqy/wqy-microhei.ttc",
    // Ubuntu 18.04+
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansSC-Regular.otf",
    "/usr/share/fonts/truetype/noto/NotoSansSC-Regular.ttf",
  ];
  for path in candidates {
    if let Ok(data) = std::fs::read(path) {
      fonts.font_data.insert(CJK_FONT.into(), FontData::from_owned(data).into());
      let prop = fonts.families.entry(FontFamily::Proportional).or_default();
      prop.insert(0, CJK_FONT.into());
      let mono = fonts.families.entry(FontFamily::Monospace).or_default();
      mono.insert(0, CJK_FONT.into());
      break;
    }
  }
  ctx.set_fonts(fonts);
}
