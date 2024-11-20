pub const CONTAINER: &str = "bg-gray-900 container mx-auto px-6 py-10 max-w-4xl rounded-xl shadow-lg mt-16";
pub const CONTAINER_SM: &str = "container mx-auto px-6 py-10 max-w-2xl rounded-xl shadow-lg mt-16";

pub const CARD: &str = "bg-gray-800 border border-gray-700 rounded-lg shadow-md p-6 max-w-xl mx-auto mt-16";
pub const CARD_HOVER_SCALE: &str = "bg-gray-800 border border-gray-700 rounded-lg shadow-md p-6 transform transition-transform duration-200 hover:scale-105";
pub const CARD_SECTION: &str = "bg-gray-800 border border-gray-700 p-3 rounded-lg shadow-sm";
pub const ALERT_CARD: &str = "p-4 rounded-lg shadow-md mb-6";

pub const INPUT_BASE: &str = "appearance-none border border-gray-600 bg-gray-800 text-white text-lg rounded-md w-full py-2 px-4 focus:outline-none focus:border-blue-500";
pub const INPUT_GROUP: &str = "flex-1 flex flex-col gap-2";

pub const BUTTON_BASE: &str = "px-5 py-2 rounded-lg font-medium text-white transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed";
pub const BUTTON_PRIMARY: &str = "bg-blue-600 hover:bg-blue-700 focus:ring-2 focus:ring-blue-400 focus:outline-none";
pub const BUTTON_SUCCESS: &str = "bg-green-600 hover:bg-green-700 focus:ring-2 focus:ring-green-400 focus:outline-none";
pub const BUTTON_WARNING: &str = "bg-yellow-600 hover:bg-yellow-700 focus:ring-2 focus:ring-yellow-400 focus:outline-none";
pub const BUTTON_DANGER: &str = "bg-red-600 hover:bg-red-700 focus:ring-2 focus:ring-red-400 focus:outline-none";
pub const BUTTON_FULL: &str = "w-full py-3 px-5 font-semibold rounded-lg transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed mt-8";

pub const TEXT_LABEL: &str = "block text-sm font-semibold text-gray-200";
pub const TEXT_LABEL_SM: &str = "block text-xs font-medium text-gray-400 mb-2";
pub const TEXT_ERROR: &str = "text-sm text-red-500 font-semibold";
pub const TEXT_MUTED: &str = "text-sm text-gray-400";
pub const HEADING_LG: &str = "text-3xl font-extrabold mb-4 text-center text-gray-100";
pub const HEADING_MD: &str = "text-2xl font-bold mb-5 text-gray-100";
pub const HEADING_SM: &str = "text-xl font-semibold mb-3 text-gray-100";

pub const FLEX_BETWEEN: &str = "flex justify-between items-center";
pub const GRID_COLS_3: &str = "grid grid-cols-3 gap-4";
pub const SPACE_Y_BASE: &str = "space-y-3";
pub const SPACE_Y_LG: &str = "space-y-6";

pub const STATS_CARD: &str = "p-4 rounded-lg border shadow-sm mb-2";
pub const STATS_CARD_SUCCESS: &str = "bg-green-900 border-green-700 text-green-200 mb-2";
pub const STATS_CARD_INFO: &str = "bg-blue-900 border-blue-700 text-blue-200 mb-2";
pub const STATS_CARD_WARNING: &str = "bg-yellow-900 border-yellow-700 text-yellow-200 mb-2";

pub const BG_PAGE: &str = "bg-gray-900 min-h-screen";
pub const MEGA_PULSE: &str = "text-green-300 mega-pulse";

pub fn combine_classes(base: &str, additional: &str) -> String {
    format!("{} {}", base, additional)
}

pub fn button_primary(full_width: bool) -> String {
    if full_width {
        combine_classes(BUTTON_BASE, &combine_classes(BUTTON_PRIMARY, BUTTON_FULL))
    } else {
        combine_classes(BUTTON_BASE, BUTTON_PRIMARY)
    }
}

pub fn alert_style(style: &str) -> String {
    match style {
        "error" => combine_classes(ALERT_CARD, "bg-red-500 text-white shadow-lg"),
        "success" => combine_classes(ALERT_CARD, "bg-green-500 text-white shadow-lg"),
        "warning" => combine_classes(ALERT_CARD, "bg-yellow-500 text-white shadow-lg"),
        _ => combine_classes(ALERT_CARD, "bg-blue-500 text-white shadow-lg"),
    }
}