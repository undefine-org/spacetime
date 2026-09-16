#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 345
#define LARGE_STATE_COUNT 5
#define SYMBOL_COUNT 334
#define ALIAS_COUNT 0
#define TOKEN_COUNT 287
#define EXTERNAL_TOKEN_COUNT 1
#define FIELD_COUNT 1
#define MAX_ALIAS_SEQUENCE_LENGTH 6
#define PRODUCTION_ID_COUNT 2

enum ts_symbol_identifiers {
  sym_identifier = 1,
  anon_sym_toast = 2,
  anon_sym_wvisible = 3,
  anon_sym_when = 4,
  anon_sym_capture = 5,
  anon_sym_wait_until = 6,
  anon_sym_then = 7,
  anon_sym_mouse = 8,
  anon_sym_drawer = 9,
  anon_sym_find = 10,
  anon_sym_cart_DASHitems = 11,
  anon_sym_mutate = 12,
  anon_sym_mobile_DASHheading = 13,
  anon_sym_container = 14,
  anon_sym_view = 15,
  anon_sym_presence = 16,
  anon_sym_wait_DASHfor = 17,
  anon_sym_data = 18,
  anon_sym_example = 19,
  anon_sym_transition = 20,
  anon_sym_mount = 21,
  anon_sym_font = 22,
  anon_sym_react_DASHto_DASHintensity = 23,
  anon_sym_state = 24,
  anon_sym_clock = 25,
  anon_sym_landscape = 26,
  anon_sym_on_DASHvisible = 27,
  anon_sym_camera_DASHscan = 28,
  anon_sym_location = 29,
  anon_sym_time = 30,
  anon_sym_apply = 31,
  anon_sym_realtime = 32,
  anon_sym_clear_DASHmocks = 33,
  anon_sym_mock_DASHresponse = 34,
  anon_sym_enable_signal_tracking = 35,
  anon_sym_fade_DASHin_DASHup = 36,
  anon_sym_socket = 37,
  anon_sym_input = 38,
  anon_sym_morph_DASHto_DASHclick = 39,
  anon_sym_replace = 40,
  anon_sym_pull_DASHrefresh = 41,
  anon_sym_distort = 42,
  anon_sym_shader = 43,
  anon_sym_given = 44,
  anon_sym_pan = 45,
  anon_sym_assert = 46,
  anon_sym_mobile_DASHtokens = 47,
  anon_sym_touch_DASHlong_DASHpress = 48,
  anon_sym_preset = 49,
  anon_sym_swipe = 50,
  anon_sym_editable_DASHmark = 51,
  anon_sym_wload = 52,
  anon_sym_observe = 53,
  anon_sym_long_DASHpress = 54,
  anon_sym_form = 55,
  anon_sym_cursor = 56,
  anon_sym_smooth_DASHscroll = 57,
  anon_sym_whover = 58,
  anon_sym_on = 59,
  anon_sym_mobile_DASHswipe = 60,
  anon_sym_mobile_DASHtext = 61,
  anon_sym_reveal = 62,
  anon_sym_on_DASHmutation = 63,
  anon_sym_ios = 64,
  anon_sym_capture_state_report = 65,
  anon_sym_program = 66,
  anon_sym_media = 67,
  anon_sym_snapshot = 68,
  anon_sym_test_DASHskip = 69,
  anon_sym_fade_DASHin_DASHdown = 70,
  anon_sym_eval = 71,
  anon_sym_mobile_DASHmotion = 72,
  anon_sym_wscroll = 73,
  anon_sym_else = 74,
  anon_sym_match = 75,
  anon_sym_resize = 76,
  anon_sym_each = 77,
  anon_sym_mobile_DASHinput = 78,
  anon_sym_test_list = 79,
  anon_sym_after = 80,
  anon_sym_morph_DASHto_DASHradius = 81,
  anon_sym_magnetic_DASHglow = 82,
  anon_sym_fill = 83,
  anon_sym_import = 84,
  anon_sym_particle_DASHfield = 85,
  anon_sym_record_DASHtimeline = 86,
  anon_sym_assert_DASHmock_DASHcalled = 87,
  anon_sym_effect = 88,
  anon_sym_video_DASHvisibility = 89,
  anon_sym_wloop = 90,
  anon_sym_biometric = 91,
  anon_sym_touch_DASHpinch = 92,
  anon_sym_notification = 93,
  anon_sym_sequence = 94,
  anon_sym_safe_DASHarea_DASHcontainer = 95,
  anon_sym_mobile_DASHtextarea = 96,
  anon_sym_breakpoint = 97,
  anon_sym_pinch = 98,
  anon_sym_touch_DASHstate = 99,
  anon_sym_run = 100,
  anon_sym_sheet = 101,
  anon_sym_richtext = 102,
  anon_sym_morph_DASHto_DASHintensity = 103,
  anon_sym_fn = 104,
  anon_sym_on_DASHhover = 105,
  anon_sym_scroll = 106,
  anon_sym_replace_DASHregex = 107,
  anon_sym_enable_timing = 108,
  anon_sym_in = 109,
  anon_sym_modal = 110,
  anon_sym_type = 111,
  anon_sym_pulse_DASHfalloff = 112,
  anon_sym_tabs = 113,
  anon_sym_mobile = 114,
  anon_sym_drag = 115,
  anon_sym_morph_DASHto_DASHcolor = 116,
  anon_sym_wait_for_signal = 117,
  anon_sym_test = 118,
  anon_sym_mock = 119,
  anon_sym_on_DASHclick = 120,
  anon_sym_cleanup = 121,
  anon_sym_timeline = 122,
  anon_sym_network_DASHset = 123,
  anon_sym_touch_DASHswipe = 124,
  anon_sym_include = 125,
  anon_sym_mobile_DASHtypography = 126,
  anon_sym_model = 127,
  anon_sym_react_DASHto_DASHcenter = 128,
  anon_sym_wait_for_state = 129,
  anon_sym_capture_signal_report = 130,
  anon_sym_camera = 131,
  anon_sym_device_DASHframe = 132,
  anon_sym_mock_DASHnative = 133,
  anon_sym_touch_DASHdrag = 134,
  anon_sym_mock_DASHpermission = 135,
  anon_sym_async_transition = 136,
  anon_sym_device = 137,
  anon_sym_mobile_DASHbutton = 138,
  anon_sym_portal = 139,
  anon_sym_mobile_DASHcolors_DASHdark = 140,
  anon_sym_click = 141,
  anon_sym_touch_DASHtap = 142,
  anon_sym_wait = 143,
  anon_sym_on_DASHfocus = 144,
  anon_sym_fuzz = 145,
  anon_sym_output_DASHjson = 146,
  anon_sym_offline = 147,
  anon_sym_fixture = 148,
  anon_sym_on_DASHintersect = 149,
  anon_sym_print = 150,
  anon_sym_out = 151,
  anon_sym_enable_state_tracking = 152,
  anon_sym_scroll_DASHview = 153,
  anon_sym_native = 154,
  anon_sym_scene = 155,
  anon_sym_state_machine_block = 156,
  anon_sym_hover = 157,
  anon_sym_haptic = 158,
  anon_sym_editable = 159,
  anon_sym_test_DASHonly = 160,
  anon_sym_device_DASHcustom = 161,
  anon_sym_mobile_DASHtheme = 162,
  anon_sym_ws_DASHon = 163,
  anon_sym_cart_DASHcount = 164,
  anon_sym_cycle = 165,
  anon_sym_for = 166,
  anon_sym_light = 167,
  anon_sym_fade_DASHin_DASHstagger = 168,
  anon_sym_morph_DASHto = 169,
  anon_sym_channel = 170,
  anon_sym_mobile_DASHsearch = 171,
  anon_sym_slow_DASHnetwork = 172,
  anon_sym_mobile_DASHinput_DASHfield = 173,
  anon_sym_if = 174,
  anon_sym_use = 175,
  anon_sym_mobile_DASHcolors = 176,
  anon_sym_script = 177,
  anon_sym_mouse_DASHposition = 178,
  anon_sym_show = 179,
  anon_sym_surface = 180,
  anon_sym_morph_DASHto_DASHfalloff = 181,
  anon_sym_property = 182,
  anon_sym_magnetic = 183,
  anon_sym_react_DASHto_DASHradius = 184,
  anon_sym_safe_DASHarea_DASHinset = 185,
  anon_sym_repeat = 186,
  anon_sym_chain = 187,
  anon_sym_locale = 188,
  anon_sym_dark = 189,
  anon_sym_compute = 190,
  anon_sym_state_machine = 191,
  anon_sym_capture_timing_report = 192,
  anon_sym_load = 193,
  anon_sym_morph_DASHto_DASHglow = 194,
  anon_sym_behavior = 195,
  anon_sym_share = 196,
  anon_sym_cart_DASHtotal = 197,
  anon_sym_value_DASHchange = 198,
  anon_sym_fade_DASHin = 199,
  anon_sym_output = 200,
  anon_sym_android = 201,
  anon_sym_loop = 202,
  anon_sym_portrait = 203,
  anon_sym_pulse_DASHradius = 204,
  anon_sym_bind = 205,
  anon_sym_try = 206,
  anon_sym_drive = 207,
  anon_sym_log = 208,
  anon_sym_scroll_DASHspy = 209,
  anon_sym_websocket = 210,
  anon_sym_let = 211,
  anon_sym_network = 212,
  anon_sym_editable_DASHblock = 213,
  anon_sym_touch_DASHpreset = 214,
  anon_sym_template = 215,
  anon_sym_swarm = 216,
  anon_sym_persist = 217,
  anon_sym_won = 218,
  anon_sym_stack = 219,
  anon_sym_pulse_DASHintensity = 220,
  anon_sym_wclick = 221,
  anon_sym_flush = 222,
  anon_sym_wait_for_timeline = 223,
  anon_sym_safe_DASHarea = 224,
  anon_sym_connect = 225,
  anon_sym_error_list = 226,
  anon_sym_reduced_DASHmotion = 227,
  anon_sym_AT = 228,
  anon_sym_SEMI = 229,
  anon_sym_LPAREN = 230,
  anon_sym_RPAREN = 231,
  anon_sym_COLON = 232,
  anon_sym_COMMA = 233,
  anon_sym_as = 234,
  anon_sym_EQ = 235,
  anon_sym_PERCENT = 236,
  anon_sym_emit = 237,
  anon_sym_LBRACE = 238,
  anon_sym_RBRACE = 239,
  anon_sym_macro = 240,
  anon_sym_primitive = 241,
  anon_sym_capture_type = 242,
  anon_sym_captureType = 243,
  anon_sym_DASH_GT = 244,
  anon_sym_PLUS = 245,
  anon_sym_DASH = 246,
  anon_sym_STAR = 247,
  anon_sym_SLASH = 248,
  anon_sym_EQ_EQ_EQ = 249,
  anon_sym_BANG_EQ_EQ = 250,
  anon_sym_EQ_EQ = 251,
  anon_sym_BANG_EQ = 252,
  anon_sym_LT = 253,
  anon_sym_GT = 254,
  anon_sym_LT_EQ = 255,
  anon_sym_GT_EQ = 256,
  anon_sym_AMP_AMP = 257,
  anon_sym_PIPE_PIPE = 258,
  anon_sym_AMP = 259,
  anon_sym_is = 260,
  anon_sym_cubic_DASHbezier = 261,
  anon_sym_QMARK = 262,
  anon_sym_from = 263,
  anon_sym_to = 264,
  anon_sym_PLUS_EQ = 265,
  anon_sym_DASH_EQ = 266,
  anon_sym_STAR_EQ = 267,
  anon_sym_SLASH_EQ = 268,
  anon_sym_SLASH_GT = 269,
  anon_sym_LT_SLASH = 270,
  anon_sym_SLASH_SLASH = 271,
  aux_sym_line_comment_token1 = 272,
  anon_sym_SLASH_STAR = 273,
  aux_sym_block_comment_token1 = 274,
  sym_property_name = 275,
  anon_sym_DOLLAR = 276,
  anon_sym_TILDE = 277,
  sym_selector = 278,
  sym_string = 279,
  sym_template_string = 280,
  sym_number = 281,
  sym_duration = 282,
  sym_dimension = 283,
  sym_percentage = 284,
  sym_color = 285,
  sym_emit_content = 286,
  sym_source_file = 287,
  sym__item = 288,
  sym__known_directive_name = 289,
  sym_directive = 290,
  sym_generic_directive = 291,
  sym__directive_args = 292,
  sym__inline_directive_args = 293,
  sym__args_inner = 294,
  sym__arg = 295,
  sym__inline_arg = 296,
  sym_emit_directive = 297,
  sym_macro_def = 298,
  sym_primitive_def = 299,
  sym_capture_type_def = 300,
  sym_meta_directive = 301,
  sym__meta_inline_args = 302,
  sym__meta_inline_arg = 303,
  sym_meta_block = 304,
  sym__meta_item = 305,
  sym_scope_block = 306,
  sym_selector_list = 307,
  sym_block = 308,
  sym__block_item = 309,
  sym_nested_scope = 310,
  sym_property_decl = 311,
  sym__property_value = 312,
  sym_transition_arrow = 313,
  sym_value_decl = 314,
  sym__value = 315,
  sym__expression = 316,
  sym_binary_expr = 317,
  sym_paren_expr = 318,
  sym_function_call = 319,
  sym_line_comment = 320,
  sym_block_comment = 321,
  sym_variable_ref = 322,
  sym_element_ref = 323,
  sym_preset_ref = 324,
  aux_sym_source_file_repeat1 = 325,
  aux_sym__inline_directive_args_repeat1 = 326,
  aux_sym__args_inner_repeat1 = 327,
  aux_sym__meta_inline_args_repeat1 = 328,
  aux_sym_meta_block_repeat1 = 329,
  aux_sym_selector_list_repeat1 = 330,
  aux_sym_selector_list_repeat2 = 331,
  aux_sym_block_repeat1 = 332,
  aux_sym__property_value_repeat1 = 333,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [sym_identifier] = "identifier",
  [anon_sym_toast] = "toast",
  [anon_sym_wvisible] = "wvisible",
  [anon_sym_when] = "when",
  [anon_sym_capture] = "capture",
  [anon_sym_wait_until] = "wait_until",
  [anon_sym_then] = "then",
  [anon_sym_mouse] = "mouse",
  [anon_sym_drawer] = "drawer",
  [anon_sym_find] = "find",
  [anon_sym_cart_DASHitems] = "cart-items",
  [anon_sym_mutate] = "mutate",
  [anon_sym_mobile_DASHheading] = "mobile-heading",
  [anon_sym_container] = "container",
  [anon_sym_view] = "view",
  [anon_sym_presence] = "presence",
  [anon_sym_wait_DASHfor] = "wait-for",
  [anon_sym_data] = "data",
  [anon_sym_example] = "example",
  [anon_sym_transition] = "transition",
  [anon_sym_mount] = "mount",
  [anon_sym_font] = "font",
  [anon_sym_react_DASHto_DASHintensity] = "react-to-intensity",
  [anon_sym_state] = "state",
  [anon_sym_clock] = "clock",
  [anon_sym_landscape] = "landscape",
  [anon_sym_on_DASHvisible] = "on-visible",
  [anon_sym_camera_DASHscan] = "camera-scan",
  [anon_sym_location] = "location",
  [anon_sym_time] = "time",
  [anon_sym_apply] = "apply",
  [anon_sym_realtime] = "realtime",
  [anon_sym_clear_DASHmocks] = "clear-mocks",
  [anon_sym_mock_DASHresponse] = "mock-response",
  [anon_sym_enable_signal_tracking] = "enable_signal_tracking",
  [anon_sym_fade_DASHin_DASHup] = "fade-in-up",
  [anon_sym_socket] = "socket",
  [anon_sym_input] = "input",
  [anon_sym_morph_DASHto_DASHclick] = "morph-to-click",
  [anon_sym_replace] = "replace",
  [anon_sym_pull_DASHrefresh] = "pull-refresh",
  [anon_sym_distort] = "distort",
  [anon_sym_shader] = "shader",
  [anon_sym_given] = "given",
  [anon_sym_pan] = "pan",
  [anon_sym_assert] = "assert",
  [anon_sym_mobile_DASHtokens] = "mobile-tokens",
  [anon_sym_touch_DASHlong_DASHpress] = "touch-long-press",
  [anon_sym_preset] = "preset",
  [anon_sym_swipe] = "swipe",
  [anon_sym_editable_DASHmark] = "editable-mark",
  [anon_sym_wload] = "wload",
  [anon_sym_observe] = "observe",
  [anon_sym_long_DASHpress] = "long-press",
  [anon_sym_form] = "form",
  [anon_sym_cursor] = "cursor",
  [anon_sym_smooth_DASHscroll] = "smooth-scroll",
  [anon_sym_whover] = "whover",
  [anon_sym_on] = "on",
  [anon_sym_mobile_DASHswipe] = "mobile-swipe",
  [anon_sym_mobile_DASHtext] = "mobile-text",
  [anon_sym_reveal] = "reveal",
  [anon_sym_on_DASHmutation] = "on-mutation",
  [anon_sym_ios] = "ios",
  [anon_sym_capture_state_report] = "capture_state_report",
  [anon_sym_program] = "program",
  [anon_sym_media] = "media",
  [anon_sym_snapshot] = "snapshot",
  [anon_sym_test_DASHskip] = "test-skip",
  [anon_sym_fade_DASHin_DASHdown] = "fade-in-down",
  [anon_sym_eval] = "eval",
  [anon_sym_mobile_DASHmotion] = "mobile-motion",
  [anon_sym_wscroll] = "wscroll",
  [anon_sym_else] = "else",
  [anon_sym_match] = "match",
  [anon_sym_resize] = "resize",
  [anon_sym_each] = "each",
  [anon_sym_mobile_DASHinput] = "mobile-input",
  [anon_sym_test_list] = "test_list",
  [anon_sym_after] = "after",
  [anon_sym_morph_DASHto_DASHradius] = "morph-to-radius",
  [anon_sym_magnetic_DASHglow] = "magnetic-glow",
  [anon_sym_fill] = "fill",
  [anon_sym_import] = "import",
  [anon_sym_particle_DASHfield] = "particle-field",
  [anon_sym_record_DASHtimeline] = "record-timeline",
  [anon_sym_assert_DASHmock_DASHcalled] = "assert-mock-called",
  [anon_sym_effect] = "effect",
  [anon_sym_video_DASHvisibility] = "video-visibility",
  [anon_sym_wloop] = "wloop",
  [anon_sym_biometric] = "biometric",
  [anon_sym_touch_DASHpinch] = "touch-pinch",
  [anon_sym_notification] = "notification",
  [anon_sym_sequence] = "sequence",
  [anon_sym_safe_DASHarea_DASHcontainer] = "safe-area-container",
  [anon_sym_mobile_DASHtextarea] = "mobile-textarea",
  [anon_sym_breakpoint] = "breakpoint",
  [anon_sym_pinch] = "pinch",
  [anon_sym_touch_DASHstate] = "touch-state",
  [anon_sym_run] = "run",
  [anon_sym_sheet] = "sheet",
  [anon_sym_richtext] = "richtext",
  [anon_sym_morph_DASHto_DASHintensity] = "morph-to-intensity",
  [anon_sym_fn] = "fn",
  [anon_sym_on_DASHhover] = "on-hover",
  [anon_sym_scroll] = "scroll",
  [anon_sym_replace_DASHregex] = "replace-regex",
  [anon_sym_enable_timing] = "enable_timing",
  [anon_sym_in] = "in",
  [anon_sym_modal] = "modal",
  [anon_sym_type] = "type",
  [anon_sym_pulse_DASHfalloff] = "pulse-falloff",
  [anon_sym_tabs] = "tabs",
  [anon_sym_mobile] = "mobile",
  [anon_sym_drag] = "drag",
  [anon_sym_morph_DASHto_DASHcolor] = "morph-to-color",
  [anon_sym_wait_for_signal] = "wait_for_signal",
  [anon_sym_test] = "test",
  [anon_sym_mock] = "mock",
  [anon_sym_on_DASHclick] = "on-click",
  [anon_sym_cleanup] = "cleanup",
  [anon_sym_timeline] = "timeline",
  [anon_sym_network_DASHset] = "network-set",
  [anon_sym_touch_DASHswipe] = "touch-swipe",
  [anon_sym_include] = "include",
  [anon_sym_mobile_DASHtypography] = "mobile-typography",
  [anon_sym_model] = "model",
  [anon_sym_react_DASHto_DASHcenter] = "react-to-center",
  [anon_sym_wait_for_state] = "wait_for_state",
  [anon_sym_capture_signal_report] = "capture_signal_report",
  [anon_sym_camera] = "camera",
  [anon_sym_device_DASHframe] = "device-frame",
  [anon_sym_mock_DASHnative] = "mock-native",
  [anon_sym_touch_DASHdrag] = "touch-drag",
  [anon_sym_mock_DASHpermission] = "mock-permission",
  [anon_sym_async_transition] = "async_transition",
  [anon_sym_device] = "device",
  [anon_sym_mobile_DASHbutton] = "mobile-button",
  [anon_sym_portal] = "portal",
  [anon_sym_mobile_DASHcolors_DASHdark] = "mobile-colors-dark",
  [anon_sym_click] = "click",
  [anon_sym_touch_DASHtap] = "touch-tap",
  [anon_sym_wait] = "wait",
  [anon_sym_on_DASHfocus] = "on-focus",
  [anon_sym_fuzz] = "fuzz",
  [anon_sym_output_DASHjson] = "output-json",
  [anon_sym_offline] = "offline",
  [anon_sym_fixture] = "fixture",
  [anon_sym_on_DASHintersect] = "on-intersect",
  [anon_sym_print] = "print",
  [anon_sym_out] = "out",
  [anon_sym_enable_state_tracking] = "enable_state_tracking",
  [anon_sym_scroll_DASHview] = "scroll-view",
  [anon_sym_native] = "native",
  [anon_sym_scene] = "scene",
  [anon_sym_state_machine_block] = "state_machine_block",
  [anon_sym_hover] = "hover",
  [anon_sym_haptic] = "haptic",
  [anon_sym_editable] = "editable",
  [anon_sym_test_DASHonly] = "test-only",
  [anon_sym_device_DASHcustom] = "device-custom",
  [anon_sym_mobile_DASHtheme] = "mobile-theme",
  [anon_sym_ws_DASHon] = "ws-on",
  [anon_sym_cart_DASHcount] = "cart-count",
  [anon_sym_cycle] = "cycle",
  [anon_sym_for] = "for",
  [anon_sym_light] = "light",
  [anon_sym_fade_DASHin_DASHstagger] = "fade-in-stagger",
  [anon_sym_morph_DASHto] = "morph-to",
  [anon_sym_channel] = "channel",
  [anon_sym_mobile_DASHsearch] = "mobile-search",
  [anon_sym_slow_DASHnetwork] = "slow-network",
  [anon_sym_mobile_DASHinput_DASHfield] = "mobile-input-field",
  [anon_sym_if] = "if",
  [anon_sym_use] = "use",
  [anon_sym_mobile_DASHcolors] = "mobile-colors",
  [anon_sym_script] = "script",
  [anon_sym_mouse_DASHposition] = "mouse-position",
  [anon_sym_show] = "show",
  [anon_sym_surface] = "surface",
  [anon_sym_morph_DASHto_DASHfalloff] = "morph-to-falloff",
  [anon_sym_property] = "property",
  [anon_sym_magnetic] = "magnetic",
  [anon_sym_react_DASHto_DASHradius] = "react-to-radius",
  [anon_sym_safe_DASHarea_DASHinset] = "safe-area-inset",
  [anon_sym_repeat] = "repeat",
  [anon_sym_chain] = "chain",
  [anon_sym_locale] = "locale",
  [anon_sym_dark] = "dark",
  [anon_sym_compute] = "compute",
  [anon_sym_state_machine] = "state_machine",
  [anon_sym_capture_timing_report] = "capture_timing_report",
  [anon_sym_load] = "load",
  [anon_sym_morph_DASHto_DASHglow] = "morph-to-glow",
  [anon_sym_behavior] = "behavior",
  [anon_sym_share] = "share",
  [anon_sym_cart_DASHtotal] = "cart-total",
  [anon_sym_value_DASHchange] = "value-change",
  [anon_sym_fade_DASHin] = "fade-in",
  [anon_sym_output] = "output",
  [anon_sym_android] = "android",
  [anon_sym_loop] = "loop",
  [anon_sym_portrait] = "portrait",
  [anon_sym_pulse_DASHradius] = "pulse-radius",
  [anon_sym_bind] = "bind",
  [anon_sym_try] = "try",
  [anon_sym_drive] = "drive",
  [anon_sym_log] = "log",
  [anon_sym_scroll_DASHspy] = "scroll-spy",
  [anon_sym_websocket] = "websocket",
  [anon_sym_let] = "let",
  [anon_sym_network] = "network",
  [anon_sym_editable_DASHblock] = "editable-block",
  [anon_sym_touch_DASHpreset] = "touch-preset",
  [anon_sym_template] = "template",
  [anon_sym_swarm] = "swarm",
  [anon_sym_persist] = "persist",
  [anon_sym_won] = "won",
  [anon_sym_stack] = "stack",
  [anon_sym_pulse_DASHintensity] = "pulse-intensity",
  [anon_sym_wclick] = "wclick",
  [anon_sym_flush] = "flush",
  [anon_sym_wait_for_timeline] = "wait_for_timeline",
  [anon_sym_safe_DASHarea] = "safe-area",
  [anon_sym_connect] = "connect",
  [anon_sym_error_list] = "error_list",
  [anon_sym_reduced_DASHmotion] = "reduced-motion",
  [anon_sym_AT] = "@",
  [anon_sym_SEMI] = ";",
  [anon_sym_LPAREN] = "(",
  [anon_sym_RPAREN] = ")",
  [anon_sym_COLON] = ":",
  [anon_sym_COMMA] = ",",
  [anon_sym_as] = "as",
  [anon_sym_EQ] = "=",
  [anon_sym_PERCENT] = "%",
  [anon_sym_emit] = "emit",
  [anon_sym_LBRACE] = "{",
  [anon_sym_RBRACE] = "}",
  [anon_sym_macro] = "macro",
  [anon_sym_primitive] = "primitive",
  [anon_sym_capture_type] = "capture_type",
  [anon_sym_captureType] = "captureType",
  [anon_sym_DASH_GT] = "->",
  [anon_sym_PLUS] = "+",
  [anon_sym_DASH] = "-",
  [anon_sym_STAR] = "*",
  [anon_sym_SLASH] = "/",
  [anon_sym_EQ_EQ_EQ] = "===",
  [anon_sym_BANG_EQ_EQ] = "!==",
  [anon_sym_EQ_EQ] = "==",
  [anon_sym_BANG_EQ] = "!=",
  [anon_sym_LT] = "<",
  [anon_sym_GT] = ">",
  [anon_sym_LT_EQ] = "<=",
  [anon_sym_GT_EQ] = ">=",
  [anon_sym_AMP_AMP] = "&&",
  [anon_sym_PIPE_PIPE] = "||",
  [anon_sym_AMP] = "&",
  [anon_sym_is] = "is",
  [anon_sym_cubic_DASHbezier] = "cubic-bezier",
  [anon_sym_QMARK] = "\?",
  [anon_sym_from] = "from",
  [anon_sym_to] = "to",
  [anon_sym_PLUS_EQ] = "+=",
  [anon_sym_DASH_EQ] = "-=",
  [anon_sym_STAR_EQ] = "*=",
  [anon_sym_SLASH_EQ] = "/=",
  [anon_sym_SLASH_GT] = "/>",
  [anon_sym_LT_SLASH] = "</",
  [anon_sym_SLASH_SLASH] = "//",
  [aux_sym_line_comment_token1] = "line_comment_token1",
  [anon_sym_SLASH_STAR] = "/*",
  [aux_sym_block_comment_token1] = "block_comment_token1",
  [sym_property_name] = "property_name",
  [anon_sym_DOLLAR] = "$",
  [anon_sym_TILDE] = "~",
  [sym_selector] = "selector",
  [sym_string] = "string",
  [sym_template_string] = "template_string",
  [sym_number] = "number",
  [sym_duration] = "duration",
  [sym_dimension] = "dimension",
  [sym_percentage] = "percentage",
  [sym_color] = "color",
  [sym_emit_content] = "emit_content",
  [sym_source_file] = "source_file",
  [sym__item] = "_item",
  [sym__known_directive_name] = "_known_directive_name",
  [sym_directive] = "directive",
  [sym_generic_directive] = "generic_directive",
  [sym__directive_args] = "_directive_args",
  [sym__inline_directive_args] = "_inline_directive_args",
  [sym__args_inner] = "_args_inner",
  [sym__arg] = "_arg",
  [sym__inline_arg] = "_inline_arg",
  [sym_emit_directive] = "emit_directive",
  [sym_macro_def] = "macro_def",
  [sym_primitive_def] = "primitive_def",
  [sym_capture_type_def] = "capture_type_def",
  [sym_meta_directive] = "meta_directive",
  [sym__meta_inline_args] = "_meta_inline_args",
  [sym__meta_inline_arg] = "_meta_inline_arg",
  [sym_meta_block] = "meta_block",
  [sym__meta_item] = "_meta_item",
  [sym_scope_block] = "scope_block",
  [sym_selector_list] = "selector_list",
  [sym_block] = "block",
  [sym__block_item] = "_block_item",
  [sym_nested_scope] = "nested_scope",
  [sym_property_decl] = "property_decl",
  [sym__property_value] = "_property_value",
  [sym_transition_arrow] = "transition_arrow",
  [sym_value_decl] = "value_decl",
  [sym__value] = "_value",
  [sym__expression] = "_expression",
  [sym_binary_expr] = "binary_expr",
  [sym_paren_expr] = "paren_expr",
  [sym_function_call] = "function_call",
  [sym_line_comment] = "line_comment",
  [sym_block_comment] = "block_comment",
  [sym_variable_ref] = "variable_ref",
  [sym_element_ref] = "element_ref",
  [sym_preset_ref] = "preset_ref",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym__inline_directive_args_repeat1] = "_inline_directive_args_repeat1",
  [aux_sym__args_inner_repeat1] = "_args_inner_repeat1",
  [aux_sym__meta_inline_args_repeat1] = "_meta_inline_args_repeat1",
  [aux_sym_meta_block_repeat1] = "meta_block_repeat1",
  [aux_sym_selector_list_repeat1] = "selector_list_repeat1",
  [aux_sym_selector_list_repeat2] = "selector_list_repeat2",
  [aux_sym_block_repeat1] = "block_repeat1",
  [aux_sym__property_value_repeat1] = "_property_value_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [sym_identifier] = sym_identifier,
  [anon_sym_toast] = anon_sym_toast,
  [anon_sym_wvisible] = anon_sym_wvisible,
  [anon_sym_when] = anon_sym_when,
  [anon_sym_capture] = anon_sym_capture,
  [anon_sym_wait_until] = anon_sym_wait_until,
  [anon_sym_then] = anon_sym_then,
  [anon_sym_mouse] = anon_sym_mouse,
  [anon_sym_drawer] = anon_sym_drawer,
  [anon_sym_find] = anon_sym_find,
  [anon_sym_cart_DASHitems] = anon_sym_cart_DASHitems,
  [anon_sym_mutate] = anon_sym_mutate,
  [anon_sym_mobile_DASHheading] = anon_sym_mobile_DASHheading,
  [anon_sym_container] = anon_sym_container,
  [anon_sym_view] = anon_sym_view,
  [anon_sym_presence] = anon_sym_presence,
  [anon_sym_wait_DASHfor] = anon_sym_wait_DASHfor,
  [anon_sym_data] = anon_sym_data,
  [anon_sym_example] = anon_sym_example,
  [anon_sym_transition] = anon_sym_transition,
  [anon_sym_mount] = anon_sym_mount,
  [anon_sym_font] = anon_sym_font,
  [anon_sym_react_DASHto_DASHintensity] = anon_sym_react_DASHto_DASHintensity,
  [anon_sym_state] = anon_sym_state,
  [anon_sym_clock] = anon_sym_clock,
  [anon_sym_landscape] = anon_sym_landscape,
  [anon_sym_on_DASHvisible] = anon_sym_on_DASHvisible,
  [anon_sym_camera_DASHscan] = anon_sym_camera_DASHscan,
  [anon_sym_location] = anon_sym_location,
  [anon_sym_time] = anon_sym_time,
  [anon_sym_apply] = anon_sym_apply,
  [anon_sym_realtime] = anon_sym_realtime,
  [anon_sym_clear_DASHmocks] = anon_sym_clear_DASHmocks,
  [anon_sym_mock_DASHresponse] = anon_sym_mock_DASHresponse,
  [anon_sym_enable_signal_tracking] = anon_sym_enable_signal_tracking,
  [anon_sym_fade_DASHin_DASHup] = anon_sym_fade_DASHin_DASHup,
  [anon_sym_socket] = anon_sym_socket,
  [anon_sym_input] = anon_sym_input,
  [anon_sym_morph_DASHto_DASHclick] = anon_sym_morph_DASHto_DASHclick,
  [anon_sym_replace] = anon_sym_replace,
  [anon_sym_pull_DASHrefresh] = anon_sym_pull_DASHrefresh,
  [anon_sym_distort] = anon_sym_distort,
  [anon_sym_shader] = anon_sym_shader,
  [anon_sym_given] = anon_sym_given,
  [anon_sym_pan] = anon_sym_pan,
  [anon_sym_assert] = anon_sym_assert,
  [anon_sym_mobile_DASHtokens] = anon_sym_mobile_DASHtokens,
  [anon_sym_touch_DASHlong_DASHpress] = anon_sym_touch_DASHlong_DASHpress,
  [anon_sym_preset] = anon_sym_preset,
  [anon_sym_swipe] = anon_sym_swipe,
  [anon_sym_editable_DASHmark] = anon_sym_editable_DASHmark,
  [anon_sym_wload] = anon_sym_wload,
  [anon_sym_observe] = anon_sym_observe,
  [anon_sym_long_DASHpress] = anon_sym_long_DASHpress,
  [anon_sym_form] = anon_sym_form,
  [anon_sym_cursor] = anon_sym_cursor,
  [anon_sym_smooth_DASHscroll] = anon_sym_smooth_DASHscroll,
  [anon_sym_whover] = anon_sym_whover,
  [anon_sym_on] = anon_sym_on,
  [anon_sym_mobile_DASHswipe] = anon_sym_mobile_DASHswipe,
  [anon_sym_mobile_DASHtext] = anon_sym_mobile_DASHtext,
  [anon_sym_reveal] = anon_sym_reveal,
  [anon_sym_on_DASHmutation] = anon_sym_on_DASHmutation,
  [anon_sym_ios] = anon_sym_ios,
  [anon_sym_capture_state_report] = anon_sym_capture_state_report,
  [anon_sym_program] = anon_sym_program,
  [anon_sym_media] = anon_sym_media,
  [anon_sym_snapshot] = anon_sym_snapshot,
  [anon_sym_test_DASHskip] = anon_sym_test_DASHskip,
  [anon_sym_fade_DASHin_DASHdown] = anon_sym_fade_DASHin_DASHdown,
  [anon_sym_eval] = anon_sym_eval,
  [anon_sym_mobile_DASHmotion] = anon_sym_mobile_DASHmotion,
  [anon_sym_wscroll] = anon_sym_wscroll,
  [anon_sym_else] = anon_sym_else,
  [anon_sym_match] = anon_sym_match,
  [anon_sym_resize] = anon_sym_resize,
  [anon_sym_each] = anon_sym_each,
  [anon_sym_mobile_DASHinput] = anon_sym_mobile_DASHinput,
  [anon_sym_test_list] = anon_sym_test_list,
  [anon_sym_after] = anon_sym_after,
  [anon_sym_morph_DASHto_DASHradius] = anon_sym_morph_DASHto_DASHradius,
  [anon_sym_magnetic_DASHglow] = anon_sym_magnetic_DASHglow,
  [anon_sym_fill] = anon_sym_fill,
  [anon_sym_import] = anon_sym_import,
  [anon_sym_particle_DASHfield] = anon_sym_particle_DASHfield,
  [anon_sym_record_DASHtimeline] = anon_sym_record_DASHtimeline,
  [anon_sym_assert_DASHmock_DASHcalled] = anon_sym_assert_DASHmock_DASHcalled,
  [anon_sym_effect] = anon_sym_effect,
  [anon_sym_video_DASHvisibility] = anon_sym_video_DASHvisibility,
  [anon_sym_wloop] = anon_sym_wloop,
  [anon_sym_biometric] = anon_sym_biometric,
  [anon_sym_touch_DASHpinch] = anon_sym_touch_DASHpinch,
  [anon_sym_notification] = anon_sym_notification,
  [anon_sym_sequence] = anon_sym_sequence,
  [anon_sym_safe_DASHarea_DASHcontainer] = anon_sym_safe_DASHarea_DASHcontainer,
  [anon_sym_mobile_DASHtextarea] = anon_sym_mobile_DASHtextarea,
  [anon_sym_breakpoint] = anon_sym_breakpoint,
  [anon_sym_pinch] = anon_sym_pinch,
  [anon_sym_touch_DASHstate] = anon_sym_touch_DASHstate,
  [anon_sym_run] = anon_sym_run,
  [anon_sym_sheet] = anon_sym_sheet,
  [anon_sym_richtext] = anon_sym_richtext,
  [anon_sym_morph_DASHto_DASHintensity] = anon_sym_morph_DASHto_DASHintensity,
  [anon_sym_fn] = anon_sym_fn,
  [anon_sym_on_DASHhover] = anon_sym_on_DASHhover,
  [anon_sym_scroll] = anon_sym_scroll,
  [anon_sym_replace_DASHregex] = anon_sym_replace_DASHregex,
  [anon_sym_enable_timing] = anon_sym_enable_timing,
  [anon_sym_in] = anon_sym_in,
  [anon_sym_modal] = anon_sym_modal,
  [anon_sym_type] = anon_sym_type,
  [anon_sym_pulse_DASHfalloff] = anon_sym_pulse_DASHfalloff,
  [anon_sym_tabs] = anon_sym_tabs,
  [anon_sym_mobile] = anon_sym_mobile,
  [anon_sym_drag] = anon_sym_drag,
  [anon_sym_morph_DASHto_DASHcolor] = anon_sym_morph_DASHto_DASHcolor,
  [anon_sym_wait_for_signal] = anon_sym_wait_for_signal,
  [anon_sym_test] = anon_sym_test,
  [anon_sym_mock] = anon_sym_mock,
  [anon_sym_on_DASHclick] = anon_sym_on_DASHclick,
  [anon_sym_cleanup] = anon_sym_cleanup,
  [anon_sym_timeline] = anon_sym_timeline,
  [anon_sym_network_DASHset] = anon_sym_network_DASHset,
  [anon_sym_touch_DASHswipe] = anon_sym_touch_DASHswipe,
  [anon_sym_include] = anon_sym_include,
  [anon_sym_mobile_DASHtypography] = anon_sym_mobile_DASHtypography,
  [anon_sym_model] = anon_sym_model,
  [anon_sym_react_DASHto_DASHcenter] = anon_sym_react_DASHto_DASHcenter,
  [anon_sym_wait_for_state] = anon_sym_wait_for_state,
  [anon_sym_capture_signal_report] = anon_sym_capture_signal_report,
  [anon_sym_camera] = anon_sym_camera,
  [anon_sym_device_DASHframe] = anon_sym_device_DASHframe,
  [anon_sym_mock_DASHnative] = anon_sym_mock_DASHnative,
  [anon_sym_touch_DASHdrag] = anon_sym_touch_DASHdrag,
  [anon_sym_mock_DASHpermission] = anon_sym_mock_DASHpermission,
  [anon_sym_async_transition] = anon_sym_async_transition,
  [anon_sym_device] = anon_sym_device,
  [anon_sym_mobile_DASHbutton] = anon_sym_mobile_DASHbutton,
  [anon_sym_portal] = anon_sym_portal,
  [anon_sym_mobile_DASHcolors_DASHdark] = anon_sym_mobile_DASHcolors_DASHdark,
  [anon_sym_click] = anon_sym_click,
  [anon_sym_touch_DASHtap] = anon_sym_touch_DASHtap,
  [anon_sym_wait] = anon_sym_wait,
  [anon_sym_on_DASHfocus] = anon_sym_on_DASHfocus,
  [anon_sym_fuzz] = anon_sym_fuzz,
  [anon_sym_output_DASHjson] = anon_sym_output_DASHjson,
  [anon_sym_offline] = anon_sym_offline,
  [anon_sym_fixture] = anon_sym_fixture,
  [anon_sym_on_DASHintersect] = anon_sym_on_DASHintersect,
  [anon_sym_print] = anon_sym_print,
  [anon_sym_out] = anon_sym_out,
  [anon_sym_enable_state_tracking] = anon_sym_enable_state_tracking,
  [anon_sym_scroll_DASHview] = anon_sym_scroll_DASHview,
  [anon_sym_native] = anon_sym_native,
  [anon_sym_scene] = anon_sym_scene,
  [anon_sym_state_machine_block] = anon_sym_state_machine_block,
  [anon_sym_hover] = anon_sym_hover,
  [anon_sym_haptic] = anon_sym_haptic,
  [anon_sym_editable] = anon_sym_editable,
  [anon_sym_test_DASHonly] = anon_sym_test_DASHonly,
  [anon_sym_device_DASHcustom] = anon_sym_device_DASHcustom,
  [anon_sym_mobile_DASHtheme] = anon_sym_mobile_DASHtheme,
  [anon_sym_ws_DASHon] = anon_sym_ws_DASHon,
  [anon_sym_cart_DASHcount] = anon_sym_cart_DASHcount,
  [anon_sym_cycle] = anon_sym_cycle,
  [anon_sym_for] = anon_sym_for,
  [anon_sym_light] = anon_sym_light,
  [anon_sym_fade_DASHin_DASHstagger] = anon_sym_fade_DASHin_DASHstagger,
  [anon_sym_morph_DASHto] = anon_sym_morph_DASHto,
  [anon_sym_channel] = anon_sym_channel,
  [anon_sym_mobile_DASHsearch] = anon_sym_mobile_DASHsearch,
  [anon_sym_slow_DASHnetwork] = anon_sym_slow_DASHnetwork,
  [anon_sym_mobile_DASHinput_DASHfield] = anon_sym_mobile_DASHinput_DASHfield,
  [anon_sym_if] = anon_sym_if,
  [anon_sym_use] = anon_sym_use,
  [anon_sym_mobile_DASHcolors] = anon_sym_mobile_DASHcolors,
  [anon_sym_script] = anon_sym_script,
  [anon_sym_mouse_DASHposition] = anon_sym_mouse_DASHposition,
  [anon_sym_show] = anon_sym_show,
  [anon_sym_surface] = anon_sym_surface,
  [anon_sym_morph_DASHto_DASHfalloff] = anon_sym_morph_DASHto_DASHfalloff,
  [anon_sym_property] = anon_sym_property,
  [anon_sym_magnetic] = anon_sym_magnetic,
  [anon_sym_react_DASHto_DASHradius] = anon_sym_react_DASHto_DASHradius,
  [anon_sym_safe_DASHarea_DASHinset] = anon_sym_safe_DASHarea_DASHinset,
  [anon_sym_repeat] = anon_sym_repeat,
  [anon_sym_chain] = anon_sym_chain,
  [anon_sym_locale] = anon_sym_locale,
  [anon_sym_dark] = anon_sym_dark,
  [anon_sym_compute] = anon_sym_compute,
  [anon_sym_state_machine] = anon_sym_state_machine,
  [anon_sym_capture_timing_report] = anon_sym_capture_timing_report,
  [anon_sym_load] = anon_sym_load,
  [anon_sym_morph_DASHto_DASHglow] = anon_sym_morph_DASHto_DASHglow,
  [anon_sym_behavior] = anon_sym_behavior,
  [anon_sym_share] = anon_sym_share,
  [anon_sym_cart_DASHtotal] = anon_sym_cart_DASHtotal,
  [anon_sym_value_DASHchange] = anon_sym_value_DASHchange,
  [anon_sym_fade_DASHin] = anon_sym_fade_DASHin,
  [anon_sym_output] = anon_sym_output,
  [anon_sym_android] = anon_sym_android,
  [anon_sym_loop] = anon_sym_loop,
  [anon_sym_portrait] = anon_sym_portrait,
  [anon_sym_pulse_DASHradius] = anon_sym_pulse_DASHradius,
  [anon_sym_bind] = anon_sym_bind,
  [anon_sym_try] = anon_sym_try,
  [anon_sym_drive] = anon_sym_drive,
  [anon_sym_log] = anon_sym_log,
  [anon_sym_scroll_DASHspy] = anon_sym_scroll_DASHspy,
  [anon_sym_websocket] = anon_sym_websocket,
  [anon_sym_let] = anon_sym_let,
  [anon_sym_network] = anon_sym_network,
  [anon_sym_editable_DASHblock] = anon_sym_editable_DASHblock,
  [anon_sym_touch_DASHpreset] = anon_sym_touch_DASHpreset,
  [anon_sym_template] = anon_sym_template,
  [anon_sym_swarm] = anon_sym_swarm,
  [anon_sym_persist] = anon_sym_persist,
  [anon_sym_won] = anon_sym_won,
  [anon_sym_stack] = anon_sym_stack,
  [anon_sym_pulse_DASHintensity] = anon_sym_pulse_DASHintensity,
  [anon_sym_wclick] = anon_sym_wclick,
  [anon_sym_flush] = anon_sym_flush,
  [anon_sym_wait_for_timeline] = anon_sym_wait_for_timeline,
  [anon_sym_safe_DASHarea] = anon_sym_safe_DASHarea,
  [anon_sym_connect] = anon_sym_connect,
  [anon_sym_error_list] = anon_sym_error_list,
  [anon_sym_reduced_DASHmotion] = anon_sym_reduced_DASHmotion,
  [anon_sym_AT] = anon_sym_AT,
  [anon_sym_SEMI] = anon_sym_SEMI,
  [anon_sym_LPAREN] = anon_sym_LPAREN,
  [anon_sym_RPAREN] = anon_sym_RPAREN,
  [anon_sym_COLON] = anon_sym_COLON,
  [anon_sym_COMMA] = anon_sym_COMMA,
  [anon_sym_as] = anon_sym_as,
  [anon_sym_EQ] = anon_sym_EQ,
  [anon_sym_PERCENT] = anon_sym_PERCENT,
  [anon_sym_emit] = anon_sym_emit,
  [anon_sym_LBRACE] = anon_sym_LBRACE,
  [anon_sym_RBRACE] = anon_sym_RBRACE,
  [anon_sym_macro] = anon_sym_macro,
  [anon_sym_primitive] = anon_sym_primitive,
  [anon_sym_capture_type] = anon_sym_capture_type,
  [anon_sym_captureType] = anon_sym_captureType,
  [anon_sym_DASH_GT] = anon_sym_DASH_GT,
  [anon_sym_PLUS] = anon_sym_PLUS,
  [anon_sym_DASH] = anon_sym_DASH,
  [anon_sym_STAR] = anon_sym_STAR,
  [anon_sym_SLASH] = anon_sym_SLASH,
  [anon_sym_EQ_EQ_EQ] = anon_sym_EQ_EQ_EQ,
  [anon_sym_BANG_EQ_EQ] = anon_sym_BANG_EQ_EQ,
  [anon_sym_EQ_EQ] = anon_sym_EQ_EQ,
  [anon_sym_BANG_EQ] = anon_sym_BANG_EQ,
  [anon_sym_LT] = anon_sym_LT,
  [anon_sym_GT] = anon_sym_GT,
  [anon_sym_LT_EQ] = anon_sym_LT_EQ,
  [anon_sym_GT_EQ] = anon_sym_GT_EQ,
  [anon_sym_AMP_AMP] = anon_sym_AMP_AMP,
  [anon_sym_PIPE_PIPE] = anon_sym_PIPE_PIPE,
  [anon_sym_AMP] = anon_sym_AMP,
  [anon_sym_is] = anon_sym_is,
  [anon_sym_cubic_DASHbezier] = anon_sym_cubic_DASHbezier,
  [anon_sym_QMARK] = anon_sym_QMARK,
  [anon_sym_from] = anon_sym_from,
  [anon_sym_to] = anon_sym_to,
  [anon_sym_PLUS_EQ] = anon_sym_PLUS_EQ,
  [anon_sym_DASH_EQ] = anon_sym_DASH_EQ,
  [anon_sym_STAR_EQ] = anon_sym_STAR_EQ,
  [anon_sym_SLASH_EQ] = anon_sym_SLASH_EQ,
  [anon_sym_SLASH_GT] = anon_sym_SLASH_GT,
  [anon_sym_LT_SLASH] = anon_sym_LT_SLASH,
  [anon_sym_SLASH_SLASH] = anon_sym_SLASH_SLASH,
  [aux_sym_line_comment_token1] = aux_sym_line_comment_token1,
  [anon_sym_SLASH_STAR] = anon_sym_SLASH_STAR,
  [aux_sym_block_comment_token1] = aux_sym_block_comment_token1,
  [sym_property_name] = sym_property_name,
  [anon_sym_DOLLAR] = anon_sym_DOLLAR,
  [anon_sym_TILDE] = anon_sym_TILDE,
  [sym_selector] = sym_selector,
  [sym_string] = sym_string,
  [sym_template_string] = sym_template_string,
  [sym_number] = sym_number,
  [sym_duration] = sym_duration,
  [sym_dimension] = sym_dimension,
  [sym_percentage] = sym_percentage,
  [sym_color] = sym_color,
  [sym_emit_content] = sym_emit_content,
  [sym_source_file] = sym_source_file,
  [sym__item] = sym__item,
  [sym__known_directive_name] = sym__known_directive_name,
  [sym_directive] = sym_directive,
  [sym_generic_directive] = sym_generic_directive,
  [sym__directive_args] = sym__directive_args,
  [sym__inline_directive_args] = sym__inline_directive_args,
  [sym__args_inner] = sym__args_inner,
  [sym__arg] = sym__arg,
  [sym__inline_arg] = sym__inline_arg,
  [sym_emit_directive] = sym_emit_directive,
  [sym_macro_def] = sym_macro_def,
  [sym_primitive_def] = sym_primitive_def,
  [sym_capture_type_def] = sym_capture_type_def,
  [sym_meta_directive] = sym_meta_directive,
  [sym__meta_inline_args] = sym__meta_inline_args,
  [sym__meta_inline_arg] = sym__meta_inline_arg,
  [sym_meta_block] = sym_meta_block,
  [sym__meta_item] = sym__meta_item,
  [sym_scope_block] = sym_scope_block,
  [sym_selector_list] = sym_selector_list,
  [sym_block] = sym_block,
  [sym__block_item] = sym__block_item,
  [sym_nested_scope] = sym_nested_scope,
  [sym_property_decl] = sym_property_decl,
  [sym__property_value] = sym__property_value,
  [sym_transition_arrow] = sym_transition_arrow,
  [sym_value_decl] = sym_value_decl,
  [sym__value] = sym__value,
  [sym__expression] = sym__expression,
  [sym_binary_expr] = sym_binary_expr,
  [sym_paren_expr] = sym_paren_expr,
  [sym_function_call] = sym_function_call,
  [sym_line_comment] = sym_line_comment,
  [sym_block_comment] = sym_block_comment,
  [sym_variable_ref] = sym_variable_ref,
  [sym_element_ref] = sym_element_ref,
  [sym_preset_ref] = sym_preset_ref,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym__inline_directive_args_repeat1] = aux_sym__inline_directive_args_repeat1,
  [aux_sym__args_inner_repeat1] = aux_sym__args_inner_repeat1,
  [aux_sym__meta_inline_args_repeat1] = aux_sym__meta_inline_args_repeat1,
  [aux_sym_meta_block_repeat1] = aux_sym_meta_block_repeat1,
  [aux_sym_selector_list_repeat1] = aux_sym_selector_list_repeat1,
  [aux_sym_selector_list_repeat2] = aux_sym_selector_list_repeat2,
  [aux_sym_block_repeat1] = aux_sym_block_repeat1,
  [aux_sym__property_value_repeat1] = aux_sym__property_value_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [sym_identifier] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_toast] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wvisible] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_when] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_capture] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wait_until] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_then] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mouse] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_drawer] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_find] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cart_DASHitems] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mutate] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHheading] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_container] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_view] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_presence] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wait_DASHfor] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_data] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_example] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_transition] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mount] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_font] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_react_DASHto_DASHintensity] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_state] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_clock] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_landscape] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on_DASHvisible] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_camera_DASHscan] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_location] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_time] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_apply] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_realtime] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_clear_DASHmocks] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mock_DASHresponse] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_enable_signal_tracking] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fade_DASHin_DASHup] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_socket] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_input] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto_DASHclick] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_replace] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_pull_DASHrefresh] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_distort] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_shader] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_given] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_pan] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_assert] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHtokens] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHlong_DASHpress] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_preset] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_swipe] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_editable_DASHmark] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wload] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_observe] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_long_DASHpress] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_form] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cursor] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_smooth_DASHscroll] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_whover] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHswipe] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHtext] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_reveal] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on_DASHmutation] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_ios] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_capture_state_report] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_program] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_media] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_snapshot] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_test_DASHskip] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fade_DASHin_DASHdown] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_eval] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHmotion] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wscroll] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_else] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_match] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_resize] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_each] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHinput] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_test_list] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_after] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto_DASHradius] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_magnetic_DASHglow] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fill] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_import] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_particle_DASHfield] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_record_DASHtimeline] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_assert_DASHmock_DASHcalled] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_effect] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_video_DASHvisibility] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wloop] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_biometric] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHpinch] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_notification] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_sequence] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_safe_DASHarea_DASHcontainer] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHtextarea] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_breakpoint] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_pinch] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHstate] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_run] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_sheet] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_richtext] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto_DASHintensity] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fn] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on_DASHhover] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_scroll] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_replace_DASHregex] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_enable_timing] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_in] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_modal] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_type] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_pulse_DASHfalloff] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_tabs] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_drag] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto_DASHcolor] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wait_for_signal] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_test] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mock] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on_DASHclick] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cleanup] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_timeline] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_network_DASHset] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHswipe] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_include] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHtypography] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_model] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_react_DASHto_DASHcenter] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wait_for_state] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_capture_signal_report] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_camera] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_device_DASHframe] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mock_DASHnative] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHdrag] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mock_DASHpermission] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_async_transition] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_device] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHbutton] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_portal] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHcolors_DASHdark] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_click] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHtap] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wait] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on_DASHfocus] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fuzz] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_output_DASHjson] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_offline] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fixture] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on_DASHintersect] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_print] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_out] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_enable_state_tracking] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_scroll_DASHview] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_native] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_scene] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_state_machine_block] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_hover] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_haptic] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_editable] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_test_DASHonly] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_device_DASHcustom] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHtheme] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_ws_DASHon] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cart_DASHcount] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cycle] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_for] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_light] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fade_DASHin_DASHstagger] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_channel] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHsearch] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_slow_DASHnetwork] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHinput_DASHfield] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_if] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_use] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mobile_DASHcolors] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_script] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_mouse_DASHposition] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_show] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_surface] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto_DASHfalloff] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_property] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_magnetic] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_react_DASHto_DASHradius] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_safe_DASHarea_DASHinset] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_repeat] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_chain] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_locale] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_dark] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_compute] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_state_machine] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_capture_timing_report] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_load] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_morph_DASHto_DASHglow] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_behavior] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_share] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cart_DASHtotal] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_value_DASHchange] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fade_DASHin] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_output] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_android] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_loop] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_portrait] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_pulse_DASHradius] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_bind] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_try] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_drive] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_log] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_scroll_DASHspy] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_websocket] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_let] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_network] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_editable_DASHblock] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_touch_DASHpreset] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_template] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_swarm] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_persist] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_won] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_stack] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_pulse_DASHintensity] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wclick] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_flush] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_wait_for_timeline] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_safe_DASHarea] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_connect] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_error_list] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_reduced_DASHmotion] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_AT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SEMI] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LPAREN] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RPAREN] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COLON] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COMMA] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_as] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_PERCENT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_emit] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_macro] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_primitive] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_capture_type] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_captureType] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DASH_GT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_PLUS] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DASH] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_STAR] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SLASH] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_EQ_EQ_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_BANG_EQ_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_EQ_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_BANG_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_GT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LT_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_GT_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_AMP_AMP] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_PIPE_PIPE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_AMP] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_is] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cubic_DASHbezier] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_QMARK] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_from] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_to] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_PLUS_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DASH_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_STAR_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SLASH_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SLASH_GT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LT_SLASH] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SLASH_SLASH] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_line_comment_token1] = {
    .visible = false,
    .named = false,
  },
  [anon_sym_SLASH_STAR] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_block_comment_token1] = {
    .visible = false,
    .named = false,
  },
  [sym_property_name] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_DOLLAR] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_TILDE] = {
    .visible = true,
    .named = false,
  },
  [sym_selector] = {
    .visible = true,
    .named = true,
  },
  [sym_string] = {
    .visible = true,
    .named = true,
  },
  [sym_template_string] = {
    .visible = true,
    .named = true,
  },
  [sym_number] = {
    .visible = true,
    .named = true,
  },
  [sym_duration] = {
    .visible = true,
    .named = true,
  },
  [sym_dimension] = {
    .visible = true,
    .named = true,
  },
  [sym_percentage] = {
    .visible = true,
    .named = true,
  },
  [sym_color] = {
    .visible = true,
    .named = true,
  },
  [sym_emit_content] = {
    .visible = true,
    .named = true,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym__item] = {
    .visible = false,
    .named = true,
  },
  [sym__known_directive_name] = {
    .visible = false,
    .named = true,
  },
  [sym_directive] = {
    .visible = true,
    .named = true,
  },
  [sym_generic_directive] = {
    .visible = true,
    .named = true,
  },
  [sym__directive_args] = {
    .visible = false,
    .named = true,
  },
  [sym__inline_directive_args] = {
    .visible = false,
    .named = true,
  },
  [sym__args_inner] = {
    .visible = false,
    .named = true,
  },
  [sym__arg] = {
    .visible = false,
    .named = true,
  },
  [sym__inline_arg] = {
    .visible = false,
    .named = true,
  },
  [sym_emit_directive] = {
    .visible = true,
    .named = true,
  },
  [sym_macro_def] = {
    .visible = true,
    .named = true,
  },
  [sym_primitive_def] = {
    .visible = true,
    .named = true,
  },
  [sym_capture_type_def] = {
    .visible = true,
    .named = true,
  },
  [sym_meta_directive] = {
    .visible = true,
    .named = true,
  },
  [sym__meta_inline_args] = {
    .visible = false,
    .named = true,
  },
  [sym__meta_inline_arg] = {
    .visible = false,
    .named = true,
  },
  [sym_meta_block] = {
    .visible = true,
    .named = true,
  },
  [sym__meta_item] = {
    .visible = false,
    .named = true,
  },
  [sym_scope_block] = {
    .visible = true,
    .named = true,
  },
  [sym_selector_list] = {
    .visible = true,
    .named = true,
  },
  [sym_block] = {
    .visible = true,
    .named = true,
  },
  [sym__block_item] = {
    .visible = false,
    .named = true,
  },
  [sym_nested_scope] = {
    .visible = true,
    .named = true,
  },
  [sym_property_decl] = {
    .visible = true,
    .named = true,
  },
  [sym__property_value] = {
    .visible = false,
    .named = true,
  },
  [sym_transition_arrow] = {
    .visible = true,
    .named = true,
  },
  [sym_value_decl] = {
    .visible = true,
    .named = true,
  },
  [sym__value] = {
    .visible = false,
    .named = true,
  },
  [sym__expression] = {
    .visible = false,
    .named = true,
  },
  [sym_binary_expr] = {
    .visible = true,
    .named = true,
  },
  [sym_paren_expr] = {
    .visible = true,
    .named = true,
  },
  [sym_function_call] = {
    .visible = true,
    .named = true,
  },
  [sym_line_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_block_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_variable_ref] = {
    .visible = true,
    .named = true,
  },
  [sym_element_ref] = {
    .visible = true,
    .named = true,
  },
  [sym_preset_ref] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym__inline_directive_args_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym__args_inner_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym__meta_inline_args_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_meta_block_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_selector_list_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_selector_list_repeat2] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_block_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym__property_value_repeat1] = {
    .visible = false,
    .named = false,
  },
};

enum ts_field_identifiers {
  field_name = 1,
};

static const char * const ts_field_names[] = {
  [0] = NULL,
  [field_name] = "name",
};

static const TSFieldMapSlice ts_field_map_slices[PRODUCTION_ID_COUNT] = {
  [1] = {.index = 0, .length = 1},
};

static const TSFieldMapEntry ts_field_map_entries[] = {
  [0] =
    {field_name, 2},
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 2,
  [4] = 2,
  [5] = 5,
  [6] = 6,
  [7] = 7,
  [8] = 8,
  [9] = 9,
  [10] = 10,
  [11] = 8,
  [12] = 12,
  [13] = 13,
  [14] = 14,
  [15] = 15,
  [16] = 16,
  [17] = 8,
  [18] = 18,
  [19] = 19,
  [20] = 20,
  [21] = 19,
  [22] = 19,
  [23] = 23,
  [24] = 24,
  [25] = 25,
  [26] = 26,
  [27] = 26,
  [28] = 28,
  [29] = 5,
  [30] = 28,
  [31] = 28,
  [32] = 6,
  [33] = 7,
  [34] = 26,
  [35] = 23,
  [36] = 25,
  [37] = 18,
  [38] = 38,
  [39] = 9,
  [40] = 15,
  [41] = 16,
  [42] = 20,
  [43] = 12,
  [44] = 13,
  [45] = 14,
  [46] = 10,
  [47] = 24,
  [48] = 48,
  [49] = 48,
  [50] = 48,
  [51] = 38,
  [52] = 52,
  [53] = 53,
  [54] = 54,
  [55] = 55,
  [56] = 53,
  [57] = 57,
  [58] = 57,
  [59] = 57,
  [60] = 57,
  [61] = 6,
  [62] = 62,
  [63] = 57,
  [64] = 54,
  [65] = 55,
  [66] = 52,
  [67] = 62,
  [68] = 8,
  [69] = 69,
  [70] = 19,
  [71] = 69,
  [72] = 69,
  [73] = 73,
  [74] = 73,
  [75] = 73,
  [76] = 18,
  [77] = 77,
  [78] = 77,
  [79] = 79,
  [80] = 23,
  [81] = 81,
  [82] = 6,
  [83] = 9,
  [84] = 84,
  [85] = 85,
  [86] = 5,
  [87] = 77,
  [88] = 79,
  [89] = 85,
  [90] = 81,
  [91] = 79,
  [92] = 81,
  [93] = 93,
  [94] = 85,
  [95] = 77,
  [96] = 96,
  [97] = 93,
  [98] = 84,
  [99] = 99,
  [100] = 7,
  [101] = 101,
  [102] = 13,
  [103] = 101,
  [104] = 20,
  [105] = 16,
  [106] = 6,
  [107] = 96,
  [108] = 9,
  [109] = 6,
  [110] = 23,
  [111] = 25,
  [112] = 24,
  [113] = 99,
  [114] = 9,
  [115] = 23,
  [116] = 12,
  [117] = 15,
  [118] = 10,
  [119] = 14,
  [120] = 18,
  [121] = 18,
  [122] = 9,
  [123] = 123,
  [124] = 124,
  [125] = 6,
  [126] = 23,
  [127] = 6,
  [128] = 128,
  [129] = 13,
  [130] = 9,
  [131] = 131,
  [132] = 16,
  [133] = 20,
  [134] = 20,
  [135] = 135,
  [136] = 23,
  [137] = 124,
  [138] = 138,
  [139] = 13,
  [140] = 131,
  [141] = 123,
  [142] = 16,
  [143] = 143,
  [144] = 144,
  [145] = 145,
  [146] = 143,
  [147] = 145,
  [148] = 148,
  [149] = 148,
  [150] = 150,
  [151] = 150,
  [152] = 150,
  [153] = 153,
  [154] = 154,
  [155] = 155,
  [156] = 154,
  [157] = 157,
  [158] = 157,
  [159] = 159,
  [160] = 160,
  [161] = 155,
  [162] = 159,
  [163] = 163,
  [164] = 164,
  [165] = 165,
  [166] = 166,
  [167] = 167,
  [168] = 163,
  [169] = 165,
  [170] = 170,
  [171] = 170,
  [172] = 153,
  [173] = 160,
  [174] = 167,
  [175] = 175,
  [176] = 176,
  [177] = 176,
  [178] = 175,
  [179] = 170,
  [180] = 164,
  [181] = 167,
  [182] = 182,
  [183] = 183,
  [184] = 184,
  [185] = 185,
  [186] = 186,
  [187] = 187,
  [188] = 188,
  [189] = 189,
  [190] = 189,
  [191] = 6,
  [192] = 192,
  [193] = 193,
  [194] = 194,
  [195] = 183,
  [196] = 196,
  [197] = 185,
  [198] = 198,
  [199] = 199,
  [200] = 200,
  [201] = 184,
  [202] = 182,
  [203] = 203,
  [204] = 193,
  [205] = 188,
  [206] = 206,
  [207] = 187,
  [208] = 208,
  [209] = 209,
  [210] = 210,
  [211] = 211,
  [212] = 212,
  [213] = 213,
  [214] = 214,
  [215] = 215,
  [216] = 216,
  [217] = 186,
  [218] = 218,
  [219] = 199,
  [220] = 220,
  [221] = 209,
  [222] = 222,
  [223] = 213,
  [224] = 198,
  [225] = 200,
  [226] = 226,
  [227] = 210,
  [228] = 228,
  [229] = 192,
  [230] = 230,
  [231] = 231,
  [232] = 232,
  [233] = 203,
  [234] = 208,
  [235] = 196,
  [236] = 236,
  [237] = 215,
  [238] = 230,
  [239] = 239,
  [240] = 211,
  [241] = 241,
  [242] = 170,
  [243] = 243,
  [244] = 244,
  [245] = 245,
  [246] = 246,
  [247] = 247,
  [248] = 212,
  [249] = 249,
  [250] = 250,
  [251] = 231,
  [252] = 252,
  [253] = 253,
  [254] = 254,
  [255] = 255,
  [256] = 256,
  [257] = 257,
  [258] = 258,
  [259] = 206,
  [260] = 216,
  [261] = 153,
  [262] = 214,
  [263] = 167,
  [264] = 160,
  [265] = 265,
  [266] = 266,
  [267] = 254,
  [268] = 268,
  [269] = 164,
  [270] = 252,
  [271] = 245,
  [272] = 236,
  [273] = 247,
  [274] = 246,
  [275] = 275,
  [276] = 249,
  [277] = 277,
  [278] = 253,
  [279] = 255,
  [280] = 257,
  [281] = 281,
  [282] = 282,
  [283] = 283,
  [284] = 284,
  [285] = 285,
  [286] = 286,
  [287] = 287,
  [288] = 288,
  [289] = 289,
  [290] = 290,
  [291] = 291,
  [292] = 292,
  [293] = 293,
  [294] = 294,
  [295] = 289,
  [296] = 293,
  [297] = 289,
  [298] = 294,
  [299] = 299,
  [300] = 299,
  [301] = 301,
  [302] = 302,
  [303] = 302,
  [304] = 290,
  [305] = 290,
  [306] = 306,
  [307] = 307,
  [308] = 291,
  [309] = 309,
  [310] = 310,
  [311] = 294,
  [312] = 312,
  [313] = 293,
  [314] = 314,
  [315] = 315,
  [316] = 307,
  [317] = 309,
  [318] = 307,
  [319] = 294,
  [320] = 289,
  [321] = 293,
  [322] = 306,
  [323] = 307,
  [324] = 294,
  [325] = 289,
  [326] = 293,
  [327] = 327,
  [328] = 307,
  [329] = 294,
  [330] = 289,
  [331] = 293,
  [332] = 294,
  [333] = 289,
  [334] = 293,
  [335] = 289,
  [336] = 289,
  [337] = 337,
  [338] = 338,
  [339] = 339,
  [340] = 340,
  [341] = 310,
  [342] = 337,
  [343] = 343,
  [344] = 344,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(46);
      ADVANCE_MAP(
        '!', 18,
        '"', 7,
        '#', 140,
        '$', 131,
        '%', 63,
        '&', 88,
        '(', 53,
        ')', 54,
        '*', 73,
        '+', 68,
        ',', 56,
        '-', 70,
        '/', 75,
        ':', 55,
        ';', 52,
        '<', 80,
        '=', 62,
        '>', 82,
        '?', 89,
        '@', 51,
        '`', 20,
        'a', 110,
        'i', 109,
        '{', 64,
        '|', 36,
        '}', 65,
        '~', 132,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(11);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('_' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 1:
      ADVANCE_MAP(
        '!', 18,
        '$', 131,
        '%', 63,
        '&', 88,
        '(', 53,
        '*', 72,
        '+', 67,
        ',', 56,
        '-', 71,
        '/', 74,
        ':', 55,
        ';', 52,
        '<', 81,
        '=', 19,
        '>', 82,
        '@', 51,
        'a', 126,
        'i', 122,
        '{', 64,
        '|', 36,
        '}', 65,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(1);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 2:
      ADVANCE_MAP(
        '!', 18,
        '&', 12,
        '(', 53,
        ')', 54,
        '*', 72,
        '+', 67,
        ',', 56,
        '-', 69,
        '/', 74,
        ':', 55,
        ';', 52,
        '<', 81,
        '=', 19,
        '>', 82,
        'a', 33,
        'i', 29,
        '|', 36,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(2);
      END_STATE();
    case 3:
      ADVANCE_MAP(
        '"', 7,
        '#', 140,
        '$', 131,
        '&', 87,
        '(', 53,
        '-', 113,
        '/', 14,
        ';', 52,
        '@', 51,
        '_', 111,
        '`', 20,
        '{', 64,
        '}', 65,
        '~', 132,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(3);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 4:
      ADVANCE_MAP(
        '"', 7,
        '#', 41,
        '$', 131,
        '%', 63,
        '&', 87,
        '(', 53,
        '-', 113,
        '/', 14,
        ';', 52,
        '@', 51,
        '_', 111,
        '`', 20,
        '{', 64,
        '}', 65,
        '~', 132,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(4);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 5:
      ADVANCE_MAP(
        '"', 7,
        '#', 41,
        '$', 131,
        '&', 87,
        '(', 53,
        ')', 54,
        '-', 37,
        '/', 14,
        '`', 20,
        '~', 132,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(5);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('_' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 6:
      ADVANCE_MAP(
        '"', 7,
        '$', 131,
        '%', 63,
        '&', 87,
        '(', 53,
        '-', 129,
        '/', 14,
        ':', 55,
        ';', 52,
        '@', 51,
        '_', 111,
        '`', 20,
        'a', 107,
        'i', 106,
        '{', 64,
        '}', 65,
        '~', 132,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(6);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 7:
      if (lookahead == '"') ADVANCE(142);
      if (lookahead != 0) ADVANCE(7);
      END_STATE();
    case 8:
      ADVANCE_MAP(
        '$', 131,
        '%', 63,
        '&', 87,
        '(', 53,
        ',', 56,
        '/', 14,
        ';', 52,
        '=', 61,
        '@', 51,
        'a', 126,
        'i', 122,
        '{', 64,
        '}', 65,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(8);
      if (lookahead == '-' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 9:
      ADVANCE_MAP(
        '$', 131,
        '%', 63,
        '&', 87,
        '(', 53,
        '/', 14,
        ';', 52,
        '@', 51,
        '{', 64,
        '}', 65,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(9);
      if (lookahead == '-' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 10:
      ADVANCE_MAP(
        '$', 131,
        '%', 63,
        '(', 53,
        '-', 130,
        '/', 14,
        '@', 51,
        '_', 111,
        'i', 106,
        '{', 64,
        '}', 65,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(10);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 11:
      if (lookahead == '%') ADVANCE(148);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(11);
      END_STATE();
    case 12:
      if (lookahead == '&') ADVANCE(85);
      END_STATE();
    case 13:
      if (lookahead == ')') ADVANCE(54);
      if (lookahead == ',') ADVANCE(56);
      if (lookahead == '/') ADVANCE(14);
      if (lookahead == '=') ADVANCE(61);
      if (lookahead == 'a') ADVANCE(33);
      if (lookahead == 'i') ADVANCE(29);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(13);
      END_STATE();
    case 14:
      if (lookahead == '*') ADVANCE(102);
      if (lookahead == '/') ADVANCE(96);
      END_STATE();
    case 15:
      if (lookahead == '*') ADVANCE(105);
      if (lookahead == '/') ADVANCE(17);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(15);
      if (lookahead != 0) ADVANCE(16);
      END_STATE();
    case 16:
      if (lookahead == '*') ADVANCE(105);
      if (lookahead != 0) ADVANCE(16);
      END_STATE();
    case 17:
      if (lookahead == '*') ADVANCE(103);
      if (lookahead == '/') ADVANCE(97);
      if (lookahead != 0) ADVANCE(16);
      END_STATE();
    case 18:
      if (lookahead == '=') ADVANCE(79);
      END_STATE();
    case 19:
      if (lookahead == '=') ADVANCE(78);
      END_STATE();
    case 20:
      if (lookahead == '`') ADVANCE(143);
      if (lookahead != 0) ADVANCE(20);
      END_STATE();
    case 21:
      if (lookahead == 'a') ADVANCE(35);
      if (lookahead == 'i') ADVANCE(28);
      END_STATE();
    case 22:
      if (lookahead == 'a') ADVANCE(23);
      if (lookahead == 'e') ADVANCE(26);
      END_STATE();
    case 23:
      if (lookahead == 'd') ADVANCE(147);
      END_STATE();
    case 24:
      if (lookahead == 'e') ADVANCE(25);
      END_STATE();
    case 25:
      if (lookahead == 'g') ADVANCE(147);
      END_STATE();
    case 26:
      if (lookahead == 'm') ADVANCE(147);
      END_STATE();
    case 27:
      if (lookahead == 'm') ADVANCE(21);
      if (lookahead == 'h' ||
          lookahead == 'w') ADVANCE(147);
      END_STATE();
    case 28:
      if (lookahead == 'n') ADVANCE(147);
      END_STATE();
    case 29:
      if (lookahead == 'n') ADVANCE(47);
      END_STATE();
    case 30:
      if (lookahead == 'r') ADVANCE(147);
      END_STATE();
    case 31:
      if (lookahead == 'r') ADVANCE(28);
      END_STATE();
    case 32:
      if (lookahead == 's') ADVANCE(146);
      END_STATE();
    case 33:
      if (lookahead == 's') ADVANCE(57);
      END_STATE();
    case 34:
      if (lookahead == 'u') ADVANCE(31);
      END_STATE();
    case 35:
      if (lookahead == 'x') ADVANCE(147);
      END_STATE();
    case 36:
      if (lookahead == '|') ADVANCE(86);
      END_STATE();
    case 37:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      END_STATE();
    case 38:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(145);
      END_STATE();
    case 39:
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(154);
      END_STATE();
    case 40:
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(39);
      END_STATE();
    case 41:
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(40);
      END_STATE();
    case 42:
      if (eof) ADVANCE(46);
      ADVANCE_MAP(
        '!', 18,
        '$', 131,
        '%', 63,
        '&', 12,
        '(', 53,
        ')', 54,
        '*', 72,
        '+', 67,
        '-', 69,
        '/', 74,
        '<', 81,
        '=', 19,
        '>', 82,
        '@', 51,
        'i', 109,
        '{', 64,
        '|', 36,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(42);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 43:
      if (eof) ADVANCE(46);
      ADVANCE_MAP(
        '!', 18,
        '%', 63,
        '&', 12,
        '(', 53,
        '*', 72,
        '+', 67,
        ',', 56,
        '-', 69,
        '/', 74,
        ':', 55,
        ';', 52,
        '<', 81,
        '=', 19,
        '>', 82,
        '@', 51,
        'a', 110,
        'i', 109,
        '{', 64,
        '|', 36,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(43);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 44:
      if (eof) ADVANCE(46);
      ADVANCE_MAP(
        '"', 7,
        '#', 140,
        '$', 131,
        '%', 63,
        '&', 87,
        '(', 53,
        '-', 37,
        '/', 74,
        ';', 52,
        '@', 51,
        '`', 20,
        '{', 64,
        '~', 132,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(44);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('_' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 45:
      if (eof) ADVANCE(46);
      ADVANCE_MAP(
        '"', 7,
        '$', 131,
        '%', 63,
        '&', 87,
        '(', 53,
        ',', 56,
        '-', 37,
        '/', 14,
        ':', 55,
        ';', 52,
        '=', 61,
        '@', 51,
        '`', 20,
        'a', 110,
        'i', 109,
        '{', 64,
        '~', 132,
        '#', 141,
        '.', 141,
        '[', 141,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(45);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          ('_' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 46:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 47:
      ACCEPT_TOKEN(anon_sym_in);
      END_STATE();
    case 48:
      ACCEPT_TOKEN(anon_sym_in);
      if (lookahead == '_') ADVANCE(111);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 49:
      ACCEPT_TOKEN(anon_sym_in);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(anon_sym_in);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 51:
      ACCEPT_TOKEN(anon_sym_AT);
      END_STATE();
    case 52:
      ACCEPT_TOKEN(anon_sym_SEMI);
      END_STATE();
    case 53:
      ACCEPT_TOKEN(anon_sym_LPAREN);
      END_STATE();
    case 54:
      ACCEPT_TOKEN(anon_sym_RPAREN);
      END_STATE();
    case 55:
      ACCEPT_TOKEN(anon_sym_COLON);
      END_STATE();
    case 56:
      ACCEPT_TOKEN(anon_sym_COMMA);
      END_STATE();
    case 57:
      ACCEPT_TOKEN(anon_sym_as);
      END_STATE();
    case 58:
      ACCEPT_TOKEN(anon_sym_as);
      if (lookahead == '_') ADVANCE(111);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 59:
      ACCEPT_TOKEN(anon_sym_as);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 60:
      ACCEPT_TOKEN(anon_sym_as);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 61:
      ACCEPT_TOKEN(anon_sym_EQ);
      END_STATE();
    case 62:
      ACCEPT_TOKEN(anon_sym_EQ);
      if (lookahead == '=') ADVANCE(78);
      END_STATE();
    case 63:
      ACCEPT_TOKEN(anon_sym_PERCENT);
      END_STATE();
    case 64:
      ACCEPT_TOKEN(anon_sym_LBRACE);
      END_STATE();
    case 65:
      ACCEPT_TOKEN(anon_sym_RBRACE);
      END_STATE();
    case 66:
      ACCEPT_TOKEN(anon_sym_DASH_GT);
      END_STATE();
    case 67:
      ACCEPT_TOKEN(anon_sym_PLUS);
      END_STATE();
    case 68:
      ACCEPT_TOKEN(anon_sym_PLUS);
      if (lookahead == '=') ADVANCE(90);
      END_STATE();
    case 69:
      ACCEPT_TOKEN(anon_sym_DASH);
      END_STATE();
    case 70:
      ACCEPT_TOKEN(anon_sym_DASH);
      if (lookahead == '=') ADVANCE(91);
      if (lookahead == '>') ADVANCE(66);
      END_STATE();
    case 71:
      ACCEPT_TOKEN(anon_sym_DASH);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 72:
      ACCEPT_TOKEN(anon_sym_STAR);
      END_STATE();
    case 73:
      ACCEPT_TOKEN(anon_sym_STAR);
      if (lookahead == '=') ADVANCE(92);
      END_STATE();
    case 74:
      ACCEPT_TOKEN(anon_sym_SLASH);
      if (lookahead == '*') ADVANCE(102);
      if (lookahead == '/') ADVANCE(96);
      END_STATE();
    case 75:
      ACCEPT_TOKEN(anon_sym_SLASH);
      if (lookahead == '*') ADVANCE(102);
      if (lookahead == '/') ADVANCE(96);
      if (lookahead == '=') ADVANCE(93);
      if (lookahead == '>') ADVANCE(94);
      END_STATE();
    case 76:
      ACCEPT_TOKEN(anon_sym_EQ_EQ_EQ);
      END_STATE();
    case 77:
      ACCEPT_TOKEN(anon_sym_BANG_EQ_EQ);
      END_STATE();
    case 78:
      ACCEPT_TOKEN(anon_sym_EQ_EQ);
      if (lookahead == '=') ADVANCE(76);
      END_STATE();
    case 79:
      ACCEPT_TOKEN(anon_sym_BANG_EQ);
      if (lookahead == '=') ADVANCE(77);
      END_STATE();
    case 80:
      ACCEPT_TOKEN(anon_sym_LT);
      if (lookahead == '/') ADVANCE(95);
      if (lookahead == '=') ADVANCE(83);
      END_STATE();
    case 81:
      ACCEPT_TOKEN(anon_sym_LT);
      if (lookahead == '=') ADVANCE(83);
      END_STATE();
    case 82:
      ACCEPT_TOKEN(anon_sym_GT);
      if (lookahead == '=') ADVANCE(84);
      END_STATE();
    case 83:
      ACCEPT_TOKEN(anon_sym_LT_EQ);
      END_STATE();
    case 84:
      ACCEPT_TOKEN(anon_sym_GT_EQ);
      END_STATE();
    case 85:
      ACCEPT_TOKEN(anon_sym_AMP_AMP);
      END_STATE();
    case 86:
      ACCEPT_TOKEN(anon_sym_PIPE_PIPE);
      END_STATE();
    case 87:
      ACCEPT_TOKEN(anon_sym_AMP);
      END_STATE();
    case 88:
      ACCEPT_TOKEN(anon_sym_AMP);
      if (lookahead == '&') ADVANCE(85);
      END_STATE();
    case 89:
      ACCEPT_TOKEN(anon_sym_QMARK);
      END_STATE();
    case 90:
      ACCEPT_TOKEN(anon_sym_PLUS_EQ);
      END_STATE();
    case 91:
      ACCEPT_TOKEN(anon_sym_DASH_EQ);
      END_STATE();
    case 92:
      ACCEPT_TOKEN(anon_sym_STAR_EQ);
      END_STATE();
    case 93:
      ACCEPT_TOKEN(anon_sym_SLASH_EQ);
      END_STATE();
    case 94:
      ACCEPT_TOKEN(anon_sym_SLASH_GT);
      END_STATE();
    case 95:
      ACCEPT_TOKEN(anon_sym_LT_SLASH);
      END_STATE();
    case 96:
      ACCEPT_TOKEN(anon_sym_SLASH_SLASH);
      END_STATE();
    case 97:
      ACCEPT_TOKEN(anon_sym_SLASH_SLASH);
      if (lookahead == '*') ADVANCE(105);
      if (lookahead != 0) ADVANCE(16);
      END_STATE();
    case 98:
      ACCEPT_TOKEN(anon_sym_SLASH_SLASH);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(101);
      END_STATE();
    case 99:
      ACCEPT_TOKEN(aux_sym_line_comment_token1);
      if (lookahead == '*') ADVANCE(104);
      if (lookahead == '/') ADVANCE(98);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(101);
      END_STATE();
    case 100:
      ACCEPT_TOKEN(aux_sym_line_comment_token1);
      if (lookahead == '/') ADVANCE(99);
      if (lookahead == '\t' ||
          (0x0b <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(100);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead)) ADVANCE(101);
      END_STATE();
    case 101:
      ACCEPT_TOKEN(aux_sym_line_comment_token1);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(101);
      END_STATE();
    case 102:
      ACCEPT_TOKEN(anon_sym_SLASH_STAR);
      END_STATE();
    case 103:
      ACCEPT_TOKEN(anon_sym_SLASH_STAR);
      if (lookahead == '*') ADVANCE(105);
      if (lookahead != 0 &&
          lookahead != '/') ADVANCE(16);
      END_STATE();
    case 104:
      ACCEPT_TOKEN(anon_sym_SLASH_STAR);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(101);
      END_STATE();
    case 105:
      ACCEPT_TOKEN(aux_sym_block_comment_token1);
      if (lookahead == '*') ADVANCE(105);
      if (lookahead != 0 &&
          lookahead != '/') ADVANCE(16);
      END_STATE();
    case 106:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '_') ADVANCE(111);
      if (lookahead == 'n') ADVANCE(48);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 107:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '_') ADVANCE(111);
      if (lookahead == 's') ADVANCE(58);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 108:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '_') ADVANCE(111);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(108);
      END_STATE();
    case 109:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == 'n') ADVANCE(50);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 110:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == 's') ADVANCE(60);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 111:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(111);
      END_STATE();
    case 112:
      ACCEPT_TOKEN(sym_property_name);
      ADVANCE_MAP(
        '%', 147,
        '.', 38,
        'd', 117,
        'e', 119,
        'f', 123,
        'm', 125,
        'p', 128,
        'r', 115,
        's', 130,
        't', 127,
        'v', 120,
      );
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(112);
      if (lookahead == '-' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 113:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == '>') ADVANCE(66);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(112);
      if (lookahead == '-' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 114:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'a') ADVANCE(128);
      if (lookahead == 'i') ADVANCE(121);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 115:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'a') ADVANCE(116);
      if (lookahead == 'e') ADVANCE(119);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 116:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'd') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 117:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'e') ADVANCE(118);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 118:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'g') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 119:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'm') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 120:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'm') ADVANCE(114);
      if (lookahead == 'h' ||
          lookahead == 'w') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 121:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'n') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 122:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'n') ADVANCE(49);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 123:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'r') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 124:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'r') ADVANCE(121);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 125:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 's') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 126:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 's') ADVANCE(59);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 127:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'u') ADVANCE(124);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 128:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == 'x') ADVANCE(130);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 129:
      ACCEPT_TOKEN(sym_property_name);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(112);
      if (lookahead == '-' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 130:
      ACCEPT_TOKEN(sym_property_name);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(130);
      END_STATE();
    case 131:
      ACCEPT_TOKEN(anon_sym_DOLLAR);
      END_STATE();
    case 132:
      ACCEPT_TOKEN(anon_sym_TILDE);
      END_STATE();
    case 133:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(141);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 134:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(133);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 135:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(134);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 136:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(135);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 137:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(136);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 138:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(137);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 139:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(138);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 140:
      ACCEPT_TOKEN(sym_selector);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(139);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 141:
      ACCEPT_TOKEN(sym_selector);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          lookahead != ' ' &&
          lookahead != ',' &&
          lookahead != ';' &&
          lookahead != '{' &&
          lookahead != '}') ADVANCE(141);
      END_STATE();
    case 142:
      ACCEPT_TOKEN(sym_string);
      END_STATE();
    case 143:
      ACCEPT_TOKEN(sym_template_string);
      END_STATE();
    case 144:
      ACCEPT_TOKEN(sym_number);
      ADVANCE_MAP(
        '%', 147,
        '.', 38,
        'd', 24,
        'e', 26,
        'f', 30,
        'm', 32,
        'p', 35,
        'r', 22,
        's', 146,
        't', 34,
        'v', 27,
      );
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(144);
      END_STATE();
    case 145:
      ACCEPT_TOKEN(sym_number);
      ADVANCE_MAP(
        '%', 147,
        'd', 24,
        'e', 26,
        'f', 30,
        'm', 32,
        'p', 35,
        'r', 22,
        's', 146,
        't', 34,
        'v', 27,
      );
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(145);
      END_STATE();
    case 146:
      ACCEPT_TOKEN(sym_duration);
      END_STATE();
    case 147:
      ACCEPT_TOKEN(sym_dimension);
      END_STATE();
    case 148:
      ACCEPT_TOKEN(sym_percentage);
      END_STATE();
    case 149:
      ACCEPT_TOKEN(sym_color);
      END_STATE();
    case 150:
      ACCEPT_TOKEN(sym_color);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(149);
      END_STATE();
    case 151:
      ACCEPT_TOKEN(sym_color);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(150);
      END_STATE();
    case 152:
      ACCEPT_TOKEN(sym_color);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(151);
      END_STATE();
    case 153:
      ACCEPT_TOKEN(sym_color);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(152);
      END_STATE();
    case 154:
      ACCEPT_TOKEN(sym_color);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(153);
      END_STATE();
    default:
      return false;
  }
}

static bool ts_lex_keywords(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      ADVANCE_MAP(
        'a', 1,
        'b', 2,
        'c', 3,
        'd', 4,
        'e', 5,
        'f', 6,
        'g', 7,
        'h', 8,
        'i', 9,
        'l', 10,
        'm', 11,
        'n', 12,
        'o', 13,
        'p', 14,
        'r', 15,
        's', 16,
        't', 17,
        'u', 18,
        'v', 19,
        'w', 20,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      END_STATE();
    case 1:
      if (lookahead == 'f') ADVANCE(21);
      if (lookahead == 'n') ADVANCE(22);
      if (lookahead == 'p') ADVANCE(23);
      if (lookahead == 's') ADVANCE(24);
      END_STATE();
    case 2:
      if (lookahead == 'e') ADVANCE(25);
      if (lookahead == 'i') ADVANCE(26);
      if (lookahead == 'r') ADVANCE(27);
      END_STATE();
    case 3:
      if (lookahead == 'a') ADVANCE(28);
      if (lookahead == 'h') ADVANCE(29);
      if (lookahead == 'l') ADVANCE(30);
      if (lookahead == 'o') ADVANCE(31);
      if (lookahead == 'u') ADVANCE(32);
      if (lookahead == 'y') ADVANCE(33);
      END_STATE();
    case 4:
      if (lookahead == 'a') ADVANCE(34);
      if (lookahead == 'e') ADVANCE(35);
      if (lookahead == 'i') ADVANCE(36);
      if (lookahead == 'r') ADVANCE(37);
      END_STATE();
    case 5:
      ADVANCE_MAP(
        'a', 38,
        'd', 39,
        'f', 40,
        'l', 41,
        'm', 42,
        'n', 43,
        'r', 44,
        'v', 45,
        'x', 46,
      );
      END_STATE();
    case 6:
      if (lookahead == 'a') ADVANCE(47);
      if (lookahead == 'i') ADVANCE(48);
      if (lookahead == 'l') ADVANCE(49);
      if (lookahead == 'n') ADVANCE(50);
      if (lookahead == 'o') ADVANCE(51);
      if (lookahead == 'r') ADVANCE(52);
      if (lookahead == 'u') ADVANCE(53);
      END_STATE();
    case 7:
      if (lookahead == 'i') ADVANCE(54);
      END_STATE();
    case 8:
      if (lookahead == 'a') ADVANCE(55);
      if (lookahead == 'o') ADVANCE(56);
      END_STATE();
    case 9:
      if (lookahead == 'f') ADVANCE(57);
      if (lookahead == 'm') ADVANCE(58);
      if (lookahead == 'n') ADVANCE(59);
      if (lookahead == 'o') ADVANCE(60);
      if (lookahead == 's') ADVANCE(61);
      END_STATE();
    case 10:
      if (lookahead == 'a') ADVANCE(62);
      if (lookahead == 'e') ADVANCE(63);
      if (lookahead == 'i') ADVANCE(64);
      if (lookahead == 'o') ADVANCE(65);
      END_STATE();
    case 11:
      if (lookahead == 'a') ADVANCE(66);
      if (lookahead == 'e') ADVANCE(67);
      if (lookahead == 'o') ADVANCE(68);
      if (lookahead == 'u') ADVANCE(69);
      END_STATE();
    case 12:
      if (lookahead == 'a') ADVANCE(70);
      if (lookahead == 'e') ADVANCE(71);
      if (lookahead == 'o') ADVANCE(72);
      END_STATE();
    case 13:
      if (lookahead == 'b') ADVANCE(73);
      if (lookahead == 'f') ADVANCE(74);
      if (lookahead == 'n') ADVANCE(75);
      if (lookahead == 'u') ADVANCE(76);
      END_STATE();
    case 14:
      if (lookahead == 'a') ADVANCE(77);
      if (lookahead == 'e') ADVANCE(78);
      if (lookahead == 'i') ADVANCE(79);
      if (lookahead == 'o') ADVANCE(80);
      if (lookahead == 'r') ADVANCE(81);
      if (lookahead == 'u') ADVANCE(82);
      END_STATE();
    case 15:
      if (lookahead == 'e') ADVANCE(83);
      if (lookahead == 'i') ADVANCE(84);
      if (lookahead == 'u') ADVANCE(85);
      END_STATE();
    case 16:
      ADVANCE_MAP(
        'a', 86,
        'c', 87,
        'e', 88,
        'h', 89,
        'l', 90,
        'm', 91,
        'n', 92,
        'o', 93,
        't', 94,
        'u', 95,
        'w', 96,
      );
      END_STATE();
    case 17:
      if (lookahead == 'a') ADVANCE(97);
      if (lookahead == 'e') ADVANCE(98);
      if (lookahead == 'h') ADVANCE(99);
      if (lookahead == 'i') ADVANCE(100);
      if (lookahead == 'o') ADVANCE(101);
      if (lookahead == 'r') ADVANCE(102);
      if (lookahead == 'y') ADVANCE(103);
      END_STATE();
    case 18:
      if (lookahead == 's') ADVANCE(104);
      END_STATE();
    case 19:
      if (lookahead == 'a') ADVANCE(105);
      if (lookahead == 'i') ADVANCE(106);
      END_STATE();
    case 20:
      ADVANCE_MAP(
        'a', 107,
        'c', 108,
        'e', 109,
        'h', 110,
        'l', 111,
        'o', 112,
        's', 113,
        'v', 114,
      );
      END_STATE();
    case 21:
      if (lookahead == 't') ADVANCE(115);
      END_STATE();
    case 22:
      if (lookahead == 'd') ADVANCE(116);
      END_STATE();
    case 23:
      if (lookahead == 'p') ADVANCE(117);
      END_STATE();
    case 24:
      if (lookahead == 's') ADVANCE(118);
      if (lookahead == 'y') ADVANCE(119);
      END_STATE();
    case 25:
      if (lookahead == 'h') ADVANCE(120);
      END_STATE();
    case 26:
      if (lookahead == 'n') ADVANCE(121);
      if (lookahead == 'o') ADVANCE(122);
      END_STATE();
    case 27:
      if (lookahead == 'e') ADVANCE(123);
      END_STATE();
    case 28:
      if (lookahead == 'm') ADVANCE(124);
      if (lookahead == 'p') ADVANCE(125);
      if (lookahead == 'r') ADVANCE(126);
      END_STATE();
    case 29:
      if (lookahead == 'a') ADVANCE(127);
      END_STATE();
    case 30:
      if (lookahead == 'e') ADVANCE(128);
      if (lookahead == 'i') ADVANCE(129);
      if (lookahead == 'o') ADVANCE(130);
      END_STATE();
    case 31:
      if (lookahead == 'm') ADVANCE(131);
      if (lookahead == 'n') ADVANCE(132);
      END_STATE();
    case 32:
      if (lookahead == 'b') ADVANCE(133);
      if (lookahead == 'r') ADVANCE(134);
      END_STATE();
    case 33:
      if (lookahead == 'c') ADVANCE(135);
      END_STATE();
    case 34:
      if (lookahead == 'r') ADVANCE(136);
      if (lookahead == 't') ADVANCE(137);
      END_STATE();
    case 35:
      if (lookahead == 'v') ADVANCE(138);
      END_STATE();
    case 36:
      if (lookahead == 's') ADVANCE(139);
      END_STATE();
    case 37:
      if (lookahead == 'a') ADVANCE(140);
      if (lookahead == 'i') ADVANCE(141);
      END_STATE();
    case 38:
      if (lookahead == 'c') ADVANCE(142);
      END_STATE();
    case 39:
      if (lookahead == 'i') ADVANCE(143);
      END_STATE();
    case 40:
      if (lookahead == 'f') ADVANCE(144);
      END_STATE();
    case 41:
      if (lookahead == 's') ADVANCE(145);
      END_STATE();
    case 42:
      if (lookahead == 'i') ADVANCE(146);
      END_STATE();
    case 43:
      if (lookahead == 'a') ADVANCE(147);
      END_STATE();
    case 44:
      if (lookahead == 'r') ADVANCE(148);
      END_STATE();
    case 45:
      if (lookahead == 'a') ADVANCE(149);
      END_STATE();
    case 46:
      if (lookahead == 'a') ADVANCE(150);
      END_STATE();
    case 47:
      if (lookahead == 'd') ADVANCE(151);
      END_STATE();
    case 48:
      if (lookahead == 'l') ADVANCE(152);
      if (lookahead == 'n') ADVANCE(153);
      if (lookahead == 'x') ADVANCE(154);
      END_STATE();
    case 49:
      if (lookahead == 'u') ADVANCE(155);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(anon_sym_fn);
      END_STATE();
    case 51:
      if (lookahead == 'n') ADVANCE(156);
      if (lookahead == 'r') ADVANCE(157);
      END_STATE();
    case 52:
      if (lookahead == 'o') ADVANCE(158);
      END_STATE();
    case 53:
      if (lookahead == 'z') ADVANCE(159);
      END_STATE();
    case 54:
      if (lookahead == 'v') ADVANCE(160);
      END_STATE();
    case 55:
      if (lookahead == 'p') ADVANCE(161);
      END_STATE();
    case 56:
      if (lookahead == 'v') ADVANCE(162);
      END_STATE();
    case 57:
      ACCEPT_TOKEN(anon_sym_if);
      END_STATE();
    case 58:
      if (lookahead == 'p') ADVANCE(163);
      END_STATE();
    case 59:
      if (lookahead == 'c') ADVANCE(164);
      if (lookahead == 'p') ADVANCE(165);
      END_STATE();
    case 60:
      if (lookahead == 's') ADVANCE(166);
      END_STATE();
    case 61:
      ACCEPT_TOKEN(anon_sym_is);
      END_STATE();
    case 62:
      if (lookahead == 'n') ADVANCE(167);
      END_STATE();
    case 63:
      if (lookahead == 't') ADVANCE(168);
      END_STATE();
    case 64:
      if (lookahead == 'g') ADVANCE(169);
      END_STATE();
    case 65:
      if (lookahead == 'a') ADVANCE(170);
      if (lookahead == 'c') ADVANCE(171);
      if (lookahead == 'g') ADVANCE(172);
      if (lookahead == 'n') ADVANCE(173);
      if (lookahead == 'o') ADVANCE(174);
      END_STATE();
    case 66:
      if (lookahead == 'c') ADVANCE(175);
      if (lookahead == 'g') ADVANCE(176);
      if (lookahead == 't') ADVANCE(177);
      END_STATE();
    case 67:
      if (lookahead == 'd') ADVANCE(178);
      END_STATE();
    case 68:
      if (lookahead == 'b') ADVANCE(179);
      if (lookahead == 'c') ADVANCE(180);
      if (lookahead == 'd') ADVANCE(181);
      if (lookahead == 'r') ADVANCE(182);
      if (lookahead == 'u') ADVANCE(183);
      END_STATE();
    case 69:
      if (lookahead == 't') ADVANCE(184);
      END_STATE();
    case 70:
      if (lookahead == 't') ADVANCE(185);
      END_STATE();
    case 71:
      if (lookahead == 't') ADVANCE(186);
      END_STATE();
    case 72:
      if (lookahead == 't') ADVANCE(187);
      END_STATE();
    case 73:
      if (lookahead == 's') ADVANCE(188);
      END_STATE();
    case 74:
      if (lookahead == 'f') ADVANCE(189);
      END_STATE();
    case 75:
      ACCEPT_TOKEN(anon_sym_on);
      if (lookahead == '-') ADVANCE(190);
      END_STATE();
    case 76:
      if (lookahead == 't') ADVANCE(191);
      END_STATE();
    case 77:
      if (lookahead == 'n') ADVANCE(192);
      if (lookahead == 'r') ADVANCE(193);
      END_STATE();
    case 78:
      if (lookahead == 'r') ADVANCE(194);
      END_STATE();
    case 79:
      if (lookahead == 'n') ADVANCE(195);
      END_STATE();
    case 80:
      if (lookahead == 'r') ADVANCE(196);
      END_STATE();
    case 81:
      if (lookahead == 'e') ADVANCE(197);
      if (lookahead == 'i') ADVANCE(198);
      if (lookahead == 'o') ADVANCE(199);
      END_STATE();
    case 82:
      if (lookahead == 'l') ADVANCE(200);
      END_STATE();
    case 83:
      if (lookahead == 'a') ADVANCE(201);
      if (lookahead == 'c') ADVANCE(202);
      if (lookahead == 'd') ADVANCE(203);
      if (lookahead == 'p') ADVANCE(204);
      if (lookahead == 's') ADVANCE(205);
      if (lookahead == 'v') ADVANCE(206);
      END_STATE();
    case 84:
      if (lookahead == 'c') ADVANCE(207);
      END_STATE();
    case 85:
      if (lookahead == 'n') ADVANCE(208);
      END_STATE();
    case 86:
      if (lookahead == 'f') ADVANCE(209);
      END_STATE();
    case 87:
      if (lookahead == 'e') ADVANCE(210);
      if (lookahead == 'r') ADVANCE(211);
      END_STATE();
    case 88:
      if (lookahead == 'q') ADVANCE(212);
      END_STATE();
    case 89:
      if (lookahead == 'a') ADVANCE(213);
      if (lookahead == 'e') ADVANCE(214);
      if (lookahead == 'o') ADVANCE(215);
      END_STATE();
    case 90:
      if (lookahead == 'o') ADVANCE(216);
      END_STATE();
    case 91:
      if (lookahead == 'o') ADVANCE(217);
      END_STATE();
    case 92:
      if (lookahead == 'a') ADVANCE(218);
      END_STATE();
    case 93:
      if (lookahead == 'c') ADVANCE(219);
      END_STATE();
    case 94:
      if (lookahead == 'a') ADVANCE(220);
      END_STATE();
    case 95:
      if (lookahead == 'r') ADVANCE(221);
      END_STATE();
    case 96:
      if (lookahead == 'a') ADVANCE(222);
      if (lookahead == 'i') ADVANCE(223);
      END_STATE();
    case 97:
      if (lookahead == 'b') ADVANCE(224);
      END_STATE();
    case 98:
      if (lookahead == 'm') ADVANCE(225);
      if (lookahead == 's') ADVANCE(226);
      END_STATE();
    case 99:
      if (lookahead == 'e') ADVANCE(227);
      END_STATE();
    case 100:
      if (lookahead == 'm') ADVANCE(228);
      END_STATE();
    case 101:
      ACCEPT_TOKEN(anon_sym_to);
      if (lookahead == 'a') ADVANCE(229);
      if (lookahead == 'u') ADVANCE(230);
      END_STATE();
    case 102:
      if (lookahead == 'a') ADVANCE(231);
      if (lookahead == 'y') ADVANCE(232);
      END_STATE();
    case 103:
      if (lookahead == 'p') ADVANCE(233);
      END_STATE();
    case 104:
      if (lookahead == 'e') ADVANCE(234);
      END_STATE();
    case 105:
      if (lookahead == 'l') ADVANCE(235);
      END_STATE();
    case 106:
      if (lookahead == 'd') ADVANCE(236);
      if (lookahead == 'e') ADVANCE(237);
      END_STATE();
    case 107:
      if (lookahead == 'i') ADVANCE(238);
      END_STATE();
    case 108:
      if (lookahead == 'l') ADVANCE(239);
      END_STATE();
    case 109:
      if (lookahead == 'b') ADVANCE(240);
      END_STATE();
    case 110:
      if (lookahead == 'e') ADVANCE(241);
      if (lookahead == 'o') ADVANCE(242);
      END_STATE();
    case 111:
      if (lookahead == 'o') ADVANCE(243);
      END_STATE();
    case 112:
      if (lookahead == 'n') ADVANCE(244);
      END_STATE();
    case 113:
      if (lookahead == '-') ADVANCE(245);
      if (lookahead == 'c') ADVANCE(246);
      END_STATE();
    case 114:
      if (lookahead == 'i') ADVANCE(247);
      END_STATE();
    case 115:
      if (lookahead == 'e') ADVANCE(248);
      END_STATE();
    case 116:
      if (lookahead == 'r') ADVANCE(249);
      END_STATE();
    case 117:
      if (lookahead == 'l') ADVANCE(250);
      END_STATE();
    case 118:
      if (lookahead == 'e') ADVANCE(251);
      END_STATE();
    case 119:
      if (lookahead == 'n') ADVANCE(252);
      END_STATE();
    case 120:
      if (lookahead == 'a') ADVANCE(253);
      END_STATE();
    case 121:
      if (lookahead == 'd') ADVANCE(254);
      END_STATE();
    case 122:
      if (lookahead == 'm') ADVANCE(255);
      END_STATE();
    case 123:
      if (lookahead == 'a') ADVANCE(256);
      END_STATE();
    case 124:
      if (lookahead == 'e') ADVANCE(257);
      END_STATE();
    case 125:
      if (lookahead == 't') ADVANCE(258);
      END_STATE();
    case 126:
      if (lookahead == 't') ADVANCE(259);
      END_STATE();
    case 127:
      if (lookahead == 'i') ADVANCE(260);
      if (lookahead == 'n') ADVANCE(261);
      END_STATE();
    case 128:
      if (lookahead == 'a') ADVANCE(262);
      END_STATE();
    case 129:
      if (lookahead == 'c') ADVANCE(263);
      END_STATE();
    case 130:
      if (lookahead == 'c') ADVANCE(264);
      END_STATE();
    case 131:
      if (lookahead == 'p') ADVANCE(265);
      END_STATE();
    case 132:
      if (lookahead == 'n') ADVANCE(266);
      if (lookahead == 't') ADVANCE(267);
      END_STATE();
    case 133:
      if (lookahead == 'i') ADVANCE(268);
      END_STATE();
    case 134:
      if (lookahead == 's') ADVANCE(269);
      END_STATE();
    case 135:
      if (lookahead == 'l') ADVANCE(270);
      END_STATE();
    case 136:
      if (lookahead == 'k') ADVANCE(271);
      END_STATE();
    case 137:
      if (lookahead == 'a') ADVANCE(272);
      END_STATE();
    case 138:
      if (lookahead == 'i') ADVANCE(273);
      END_STATE();
    case 139:
      if (lookahead == 't') ADVANCE(274);
      END_STATE();
    case 140:
      if (lookahead == 'g') ADVANCE(275);
      if (lookahead == 'w') ADVANCE(276);
      END_STATE();
    case 141:
      if (lookahead == 'v') ADVANCE(277);
      END_STATE();
    case 142:
      if (lookahead == 'h') ADVANCE(278);
      END_STATE();
    case 143:
      if (lookahead == 't') ADVANCE(279);
      END_STATE();
    case 144:
      if (lookahead == 'e') ADVANCE(280);
      END_STATE();
    case 145:
      if (lookahead == 'e') ADVANCE(281);
      END_STATE();
    case 146:
      if (lookahead == 't') ADVANCE(282);
      END_STATE();
    case 147:
      if (lookahead == 'b') ADVANCE(283);
      END_STATE();
    case 148:
      if (lookahead == 'o') ADVANCE(284);
      END_STATE();
    case 149:
      if (lookahead == 'l') ADVANCE(285);
      END_STATE();
    case 150:
      if (lookahead == 'm') ADVANCE(286);
      END_STATE();
    case 151:
      if (lookahead == 'e') ADVANCE(287);
      END_STATE();
    case 152:
      if (lookahead == 'l') ADVANCE(288);
      END_STATE();
    case 153:
      if (lookahead == 'd') ADVANCE(289);
      END_STATE();
    case 154:
      if (lookahead == 't') ADVANCE(290);
      END_STATE();
    case 155:
      if (lookahead == 's') ADVANCE(291);
      END_STATE();
    case 156:
      if (lookahead == 't') ADVANCE(292);
      END_STATE();
    case 157:
      ACCEPT_TOKEN(anon_sym_for);
      if (lookahead == 'm') ADVANCE(293);
      END_STATE();
    case 158:
      if (lookahead == 'm') ADVANCE(294);
      END_STATE();
    case 159:
      if (lookahead == 'z') ADVANCE(295);
      END_STATE();
    case 160:
      if (lookahead == 'e') ADVANCE(296);
      END_STATE();
    case 161:
      if (lookahead == 't') ADVANCE(297);
      END_STATE();
    case 162:
      if (lookahead == 'e') ADVANCE(298);
      END_STATE();
    case 163:
      if (lookahead == 'o') ADVANCE(299);
      END_STATE();
    case 164:
      if (lookahead == 'l') ADVANCE(300);
      END_STATE();
    case 165:
      if (lookahead == 'u') ADVANCE(301);
      END_STATE();
    case 166:
      ACCEPT_TOKEN(anon_sym_ios);
      END_STATE();
    case 167:
      if (lookahead == 'd') ADVANCE(302);
      END_STATE();
    case 168:
      ACCEPT_TOKEN(anon_sym_let);
      END_STATE();
    case 169:
      if (lookahead == 'h') ADVANCE(303);
      END_STATE();
    case 170:
      if (lookahead == 'd') ADVANCE(304);
      END_STATE();
    case 171:
      if (lookahead == 'a') ADVANCE(305);
      END_STATE();
    case 172:
      ACCEPT_TOKEN(anon_sym_log);
      END_STATE();
    case 173:
      if (lookahead == 'g') ADVANCE(306);
      END_STATE();
    case 174:
      if (lookahead == 'p') ADVANCE(307);
      END_STATE();
    case 175:
      if (lookahead == 'r') ADVANCE(308);
      END_STATE();
    case 176:
      if (lookahead == 'n') ADVANCE(309);
      END_STATE();
    case 177:
      if (lookahead == 'c') ADVANCE(310);
      END_STATE();
    case 178:
      if (lookahead == 'i') ADVANCE(311);
      END_STATE();
    case 179:
      if (lookahead == 'i') ADVANCE(312);
      END_STATE();
    case 180:
      if (lookahead == 'k') ADVANCE(313);
      END_STATE();
    case 181:
      if (lookahead == 'a') ADVANCE(314);
      if (lookahead == 'e') ADVANCE(315);
      END_STATE();
    case 182:
      if (lookahead == 'p') ADVANCE(316);
      END_STATE();
    case 183:
      if (lookahead == 'n') ADVANCE(317);
      if (lookahead == 's') ADVANCE(318);
      END_STATE();
    case 184:
      if (lookahead == 'a') ADVANCE(319);
      END_STATE();
    case 185:
      if (lookahead == 'i') ADVANCE(320);
      END_STATE();
    case 186:
      if (lookahead == 'w') ADVANCE(321);
      END_STATE();
    case 187:
      if (lookahead == 'i') ADVANCE(322);
      END_STATE();
    case 188:
      if (lookahead == 'e') ADVANCE(323);
      END_STATE();
    case 189:
      if (lookahead == 'l') ADVANCE(324);
      END_STATE();
    case 190:
      if (lookahead == 'c') ADVANCE(325);
      if (lookahead == 'f') ADVANCE(326);
      if (lookahead == 'h') ADVANCE(327);
      if (lookahead == 'i') ADVANCE(328);
      if (lookahead == 'm') ADVANCE(329);
      if (lookahead == 'v') ADVANCE(330);
      END_STATE();
    case 191:
      ACCEPT_TOKEN(anon_sym_out);
      if (lookahead == 'p') ADVANCE(331);
      END_STATE();
    case 192:
      ACCEPT_TOKEN(anon_sym_pan);
      END_STATE();
    case 193:
      if (lookahead == 't') ADVANCE(332);
      END_STATE();
    case 194:
      if (lookahead == 's') ADVANCE(333);
      END_STATE();
    case 195:
      if (lookahead == 'c') ADVANCE(334);
      END_STATE();
    case 196:
      if (lookahead == 't') ADVANCE(335);
      END_STATE();
    case 197:
      if (lookahead == 's') ADVANCE(336);
      END_STATE();
    case 198:
      if (lookahead == 'm') ADVANCE(337);
      if (lookahead == 'n') ADVANCE(338);
      END_STATE();
    case 199:
      if (lookahead == 'g') ADVANCE(339);
      if (lookahead == 'p') ADVANCE(340);
      END_STATE();
    case 200:
      if (lookahead == 'l') ADVANCE(341);
      if (lookahead == 's') ADVANCE(342);
      END_STATE();
    case 201:
      if (lookahead == 'c') ADVANCE(343);
      if (lookahead == 'l') ADVANCE(344);
      END_STATE();
    case 202:
      if (lookahead == 'o') ADVANCE(345);
      END_STATE();
    case 203:
      if (lookahead == 'u') ADVANCE(346);
      END_STATE();
    case 204:
      if (lookahead == 'e') ADVANCE(347);
      if (lookahead == 'l') ADVANCE(348);
      END_STATE();
    case 205:
      if (lookahead == 'i') ADVANCE(349);
      END_STATE();
    case 206:
      if (lookahead == 'e') ADVANCE(350);
      END_STATE();
    case 207:
      if (lookahead == 'h') ADVANCE(351);
      END_STATE();
    case 208:
      ACCEPT_TOKEN(anon_sym_run);
      END_STATE();
    case 209:
      if (lookahead == 'e') ADVANCE(352);
      END_STATE();
    case 210:
      if (lookahead == 'n') ADVANCE(353);
      END_STATE();
    case 211:
      if (lookahead == 'i') ADVANCE(354);
      if (lookahead == 'o') ADVANCE(355);
      END_STATE();
    case 212:
      if (lookahead == 'u') ADVANCE(356);
      END_STATE();
    case 213:
      if (lookahead == 'd') ADVANCE(357);
      if (lookahead == 'r') ADVANCE(358);
      END_STATE();
    case 214:
      if (lookahead == 'e') ADVANCE(359);
      END_STATE();
    case 215:
      if (lookahead == 'w') ADVANCE(360);
      END_STATE();
    case 216:
      if (lookahead == 'w') ADVANCE(361);
      END_STATE();
    case 217:
      if (lookahead == 'o') ADVANCE(362);
      END_STATE();
    case 218:
      if (lookahead == 'p') ADVANCE(363);
      END_STATE();
    case 219:
      if (lookahead == 'k') ADVANCE(364);
      END_STATE();
    case 220:
      if (lookahead == 'c') ADVANCE(365);
      if (lookahead == 't') ADVANCE(366);
      END_STATE();
    case 221:
      if (lookahead == 'f') ADVANCE(367);
      END_STATE();
    case 222:
      if (lookahead == 'r') ADVANCE(368);
      END_STATE();
    case 223:
      if (lookahead == 'p') ADVANCE(369);
      END_STATE();
    case 224:
      if (lookahead == 's') ADVANCE(370);
      END_STATE();
    case 225:
      if (lookahead == 'p') ADVANCE(371);
      END_STATE();
    case 226:
      if (lookahead == 't') ADVANCE(372);
      END_STATE();
    case 227:
      if (lookahead == 'n') ADVANCE(373);
      END_STATE();
    case 228:
      if (lookahead == 'e') ADVANCE(374);
      END_STATE();
    case 229:
      if (lookahead == 's') ADVANCE(375);
      END_STATE();
    case 230:
      if (lookahead == 'c') ADVANCE(376);
      END_STATE();
    case 231:
      if (lookahead == 'n') ADVANCE(377);
      END_STATE();
    case 232:
      ACCEPT_TOKEN(anon_sym_try);
      END_STATE();
    case 233:
      if (lookahead == 'e') ADVANCE(378);
      END_STATE();
    case 234:
      ACCEPT_TOKEN(anon_sym_use);
      END_STATE();
    case 235:
      if (lookahead == 'u') ADVANCE(379);
      END_STATE();
    case 236:
      if (lookahead == 'e') ADVANCE(380);
      END_STATE();
    case 237:
      if (lookahead == 'w') ADVANCE(381);
      END_STATE();
    case 238:
      if (lookahead == 't') ADVANCE(382);
      END_STATE();
    case 239:
      if (lookahead == 'i') ADVANCE(383);
      END_STATE();
    case 240:
      if (lookahead == 's') ADVANCE(384);
      END_STATE();
    case 241:
      if (lookahead == 'n') ADVANCE(385);
      END_STATE();
    case 242:
      if (lookahead == 'v') ADVANCE(386);
      END_STATE();
    case 243:
      if (lookahead == 'a') ADVANCE(387);
      if (lookahead == 'o') ADVANCE(388);
      END_STATE();
    case 244:
      ACCEPT_TOKEN(anon_sym_won);
      END_STATE();
    case 245:
      if (lookahead == 'o') ADVANCE(389);
      END_STATE();
    case 246:
      if (lookahead == 'r') ADVANCE(390);
      END_STATE();
    case 247:
      if (lookahead == 's') ADVANCE(391);
      END_STATE();
    case 248:
      if (lookahead == 'r') ADVANCE(392);
      END_STATE();
    case 249:
      if (lookahead == 'o') ADVANCE(393);
      END_STATE();
    case 250:
      if (lookahead == 'y') ADVANCE(394);
      END_STATE();
    case 251:
      if (lookahead == 'r') ADVANCE(395);
      END_STATE();
    case 252:
      if (lookahead == 'c') ADVANCE(396);
      END_STATE();
    case 253:
      if (lookahead == 'v') ADVANCE(397);
      END_STATE();
    case 254:
      ACCEPT_TOKEN(anon_sym_bind);
      END_STATE();
    case 255:
      if (lookahead == 'e') ADVANCE(398);
      END_STATE();
    case 256:
      if (lookahead == 'k') ADVANCE(399);
      END_STATE();
    case 257:
      if (lookahead == 'r') ADVANCE(400);
      END_STATE();
    case 258:
      if (lookahead == 'u') ADVANCE(401);
      END_STATE();
    case 259:
      if (lookahead == '-') ADVANCE(402);
      END_STATE();
    case 260:
      if (lookahead == 'n') ADVANCE(403);
      END_STATE();
    case 261:
      if (lookahead == 'n') ADVANCE(404);
      END_STATE();
    case 262:
      if (lookahead == 'n') ADVANCE(405);
      if (lookahead == 'r') ADVANCE(406);
      END_STATE();
    case 263:
      if (lookahead == 'k') ADVANCE(407);
      END_STATE();
    case 264:
      if (lookahead == 'k') ADVANCE(408);
      END_STATE();
    case 265:
      if (lookahead == 'u') ADVANCE(409);
      END_STATE();
    case 266:
      if (lookahead == 'e') ADVANCE(410);
      END_STATE();
    case 267:
      if (lookahead == 'a') ADVANCE(411);
      END_STATE();
    case 268:
      if (lookahead == 'c') ADVANCE(412);
      END_STATE();
    case 269:
      if (lookahead == 'o') ADVANCE(413);
      END_STATE();
    case 270:
      if (lookahead == 'e') ADVANCE(414);
      END_STATE();
    case 271:
      ACCEPT_TOKEN(anon_sym_dark);
      END_STATE();
    case 272:
      ACCEPT_TOKEN(anon_sym_data);
      END_STATE();
    case 273:
      if (lookahead == 'c') ADVANCE(415);
      END_STATE();
    case 274:
      if (lookahead == 'o') ADVANCE(416);
      END_STATE();
    case 275:
      ACCEPT_TOKEN(anon_sym_drag);
      END_STATE();
    case 276:
      if (lookahead == 'e') ADVANCE(417);
      END_STATE();
    case 277:
      if (lookahead == 'e') ADVANCE(418);
      END_STATE();
    case 278:
      ACCEPT_TOKEN(anon_sym_each);
      END_STATE();
    case 279:
      if (lookahead == 'a') ADVANCE(419);
      END_STATE();
    case 280:
      if (lookahead == 'c') ADVANCE(420);
      END_STATE();
    case 281:
      ACCEPT_TOKEN(anon_sym_else);
      END_STATE();
    case 282:
      ACCEPT_TOKEN(anon_sym_emit);
      END_STATE();
    case 283:
      if (lookahead == 'l') ADVANCE(421);
      END_STATE();
    case 284:
      if (lookahead == 'r') ADVANCE(422);
      END_STATE();
    case 285:
      ACCEPT_TOKEN(anon_sym_eval);
      END_STATE();
    case 286:
      if (lookahead == 'p') ADVANCE(423);
      END_STATE();
    case 287:
      if (lookahead == '-') ADVANCE(424);
      END_STATE();
    case 288:
      ACCEPT_TOKEN(anon_sym_fill);
      END_STATE();
    case 289:
      ACCEPT_TOKEN(anon_sym_find);
      END_STATE();
    case 290:
      if (lookahead == 'u') ADVANCE(425);
      END_STATE();
    case 291:
      if (lookahead == 'h') ADVANCE(426);
      END_STATE();
    case 292:
      ACCEPT_TOKEN(anon_sym_font);
      END_STATE();
    case 293:
      ACCEPT_TOKEN(anon_sym_form);
      END_STATE();
    case 294:
      ACCEPT_TOKEN(anon_sym_from);
      END_STATE();
    case 295:
      ACCEPT_TOKEN(anon_sym_fuzz);
      END_STATE();
    case 296:
      if (lookahead == 'n') ADVANCE(427);
      END_STATE();
    case 297:
      if (lookahead == 'i') ADVANCE(428);
      END_STATE();
    case 298:
      if (lookahead == 'r') ADVANCE(429);
      END_STATE();
    case 299:
      if (lookahead == 'r') ADVANCE(430);
      END_STATE();
    case 300:
      if (lookahead == 'u') ADVANCE(431);
      END_STATE();
    case 301:
      if (lookahead == 't') ADVANCE(432);
      END_STATE();
    case 302:
      if (lookahead == 's') ADVANCE(433);
      END_STATE();
    case 303:
      if (lookahead == 't') ADVANCE(434);
      END_STATE();
    case 304:
      ACCEPT_TOKEN(anon_sym_load);
      END_STATE();
    case 305:
      if (lookahead == 'l') ADVANCE(435);
      if (lookahead == 't') ADVANCE(436);
      END_STATE();
    case 306:
      if (lookahead == '-') ADVANCE(437);
      END_STATE();
    case 307:
      ACCEPT_TOKEN(anon_sym_loop);
      END_STATE();
    case 308:
      if (lookahead == 'o') ADVANCE(438);
      END_STATE();
    case 309:
      if (lookahead == 'e') ADVANCE(439);
      END_STATE();
    case 310:
      if (lookahead == 'h') ADVANCE(440);
      END_STATE();
    case 311:
      if (lookahead == 'a') ADVANCE(441);
      END_STATE();
    case 312:
      if (lookahead == 'l') ADVANCE(442);
      END_STATE();
    case 313:
      ACCEPT_TOKEN(anon_sym_mock);
      if (lookahead == '-') ADVANCE(443);
      END_STATE();
    case 314:
      if (lookahead == 'l') ADVANCE(444);
      END_STATE();
    case 315:
      if (lookahead == 'l') ADVANCE(445);
      END_STATE();
    case 316:
      if (lookahead == 'h') ADVANCE(446);
      END_STATE();
    case 317:
      if (lookahead == 't') ADVANCE(447);
      END_STATE();
    case 318:
      if (lookahead == 'e') ADVANCE(448);
      END_STATE();
    case 319:
      if (lookahead == 't') ADVANCE(449);
      END_STATE();
    case 320:
      if (lookahead == 'v') ADVANCE(450);
      END_STATE();
    case 321:
      if (lookahead == 'o') ADVANCE(451);
      END_STATE();
    case 322:
      if (lookahead == 'f') ADVANCE(452);
      END_STATE();
    case 323:
      if (lookahead == 'r') ADVANCE(453);
      END_STATE();
    case 324:
      if (lookahead == 'i') ADVANCE(454);
      END_STATE();
    case 325:
      if (lookahead == 'l') ADVANCE(455);
      END_STATE();
    case 326:
      if (lookahead == 'o') ADVANCE(456);
      END_STATE();
    case 327:
      if (lookahead == 'o') ADVANCE(457);
      END_STATE();
    case 328:
      if (lookahead == 'n') ADVANCE(458);
      END_STATE();
    case 329:
      if (lookahead == 'u') ADVANCE(459);
      END_STATE();
    case 330:
      if (lookahead == 'i') ADVANCE(460);
      END_STATE();
    case 331:
      if (lookahead == 'u') ADVANCE(461);
      END_STATE();
    case 332:
      if (lookahead == 'i') ADVANCE(462);
      END_STATE();
    case 333:
      if (lookahead == 'i') ADVANCE(463);
      END_STATE();
    case 334:
      if (lookahead == 'h') ADVANCE(464);
      END_STATE();
    case 335:
      if (lookahead == 'a') ADVANCE(465);
      if (lookahead == 'r') ADVANCE(466);
      END_STATE();
    case 336:
      if (lookahead == 'e') ADVANCE(467);
      END_STATE();
    case 337:
      if (lookahead == 'i') ADVANCE(468);
      END_STATE();
    case 338:
      if (lookahead == 't') ADVANCE(469);
      END_STATE();
    case 339:
      if (lookahead == 'r') ADVANCE(470);
      END_STATE();
    case 340:
      if (lookahead == 'e') ADVANCE(471);
      END_STATE();
    case 341:
      if (lookahead == '-') ADVANCE(472);
      END_STATE();
    case 342:
      if (lookahead == 'e') ADVANCE(473);
      END_STATE();
    case 343:
      if (lookahead == 't') ADVANCE(474);
      END_STATE();
    case 344:
      if (lookahead == 't') ADVANCE(475);
      END_STATE();
    case 345:
      if (lookahead == 'r') ADVANCE(476);
      END_STATE();
    case 346:
      if (lookahead == 'c') ADVANCE(477);
      END_STATE();
    case 347:
      if (lookahead == 'a') ADVANCE(478);
      END_STATE();
    case 348:
      if (lookahead == 'a') ADVANCE(479);
      END_STATE();
    case 349:
      if (lookahead == 'z') ADVANCE(480);
      END_STATE();
    case 350:
      if (lookahead == 'a') ADVANCE(481);
      END_STATE();
    case 351:
      if (lookahead == 't') ADVANCE(482);
      END_STATE();
    case 352:
      if (lookahead == '-') ADVANCE(483);
      END_STATE();
    case 353:
      if (lookahead == 'e') ADVANCE(484);
      END_STATE();
    case 354:
      if (lookahead == 'p') ADVANCE(485);
      END_STATE();
    case 355:
      if (lookahead == 'l') ADVANCE(486);
      END_STATE();
    case 356:
      if (lookahead == 'e') ADVANCE(487);
      END_STATE();
    case 357:
      if (lookahead == 'e') ADVANCE(488);
      END_STATE();
    case 358:
      if (lookahead == 'e') ADVANCE(489);
      END_STATE();
    case 359:
      if (lookahead == 't') ADVANCE(490);
      END_STATE();
    case 360:
      ACCEPT_TOKEN(anon_sym_show);
      END_STATE();
    case 361:
      if (lookahead == '-') ADVANCE(491);
      END_STATE();
    case 362:
      if (lookahead == 't') ADVANCE(492);
      END_STATE();
    case 363:
      if (lookahead == 's') ADVANCE(493);
      END_STATE();
    case 364:
      if (lookahead == 'e') ADVANCE(494);
      END_STATE();
    case 365:
      if (lookahead == 'k') ADVANCE(495);
      END_STATE();
    case 366:
      if (lookahead == 'e') ADVANCE(496);
      END_STATE();
    case 367:
      if (lookahead == 'a') ADVANCE(497);
      END_STATE();
    case 368:
      if (lookahead == 'm') ADVANCE(498);
      END_STATE();
    case 369:
      if (lookahead == 'e') ADVANCE(499);
      END_STATE();
    case 370:
      ACCEPT_TOKEN(anon_sym_tabs);
      END_STATE();
    case 371:
      if (lookahead == 'l') ADVANCE(500);
      END_STATE();
    case 372:
      ACCEPT_TOKEN(anon_sym_test);
      if (lookahead == '-') ADVANCE(501);
      if (lookahead == '_') ADVANCE(502);
      END_STATE();
    case 373:
      ACCEPT_TOKEN(anon_sym_then);
      END_STATE();
    case 374:
      ACCEPT_TOKEN(anon_sym_time);
      if (lookahead == 'l') ADVANCE(503);
      END_STATE();
    case 375:
      if (lookahead == 't') ADVANCE(504);
      END_STATE();
    case 376:
      if (lookahead == 'h') ADVANCE(505);
      END_STATE();
    case 377:
      if (lookahead == 's') ADVANCE(506);
      END_STATE();
    case 378:
      ACCEPT_TOKEN(anon_sym_type);
      END_STATE();
    case 379:
      if (lookahead == 'e') ADVANCE(507);
      END_STATE();
    case 380:
      if (lookahead == 'o') ADVANCE(508);
      END_STATE();
    case 381:
      ACCEPT_TOKEN(anon_sym_view);
      END_STATE();
    case 382:
      ACCEPT_TOKEN(anon_sym_wait);
      if (lookahead == '-') ADVANCE(509);
      if (lookahead == '_') ADVANCE(510);
      END_STATE();
    case 383:
      if (lookahead == 'c') ADVANCE(511);
      END_STATE();
    case 384:
      if (lookahead == 'o') ADVANCE(512);
      END_STATE();
    case 385:
      ACCEPT_TOKEN(anon_sym_when);
      END_STATE();
    case 386:
      if (lookahead == 'e') ADVANCE(513);
      END_STATE();
    case 387:
      if (lookahead == 'd') ADVANCE(514);
      END_STATE();
    case 388:
      if (lookahead == 'p') ADVANCE(515);
      END_STATE();
    case 389:
      if (lookahead == 'n') ADVANCE(516);
      END_STATE();
    case 390:
      if (lookahead == 'o') ADVANCE(517);
      END_STATE();
    case 391:
      if (lookahead == 'i') ADVANCE(518);
      END_STATE();
    case 392:
      ACCEPT_TOKEN(anon_sym_after);
      END_STATE();
    case 393:
      if (lookahead == 'i') ADVANCE(519);
      END_STATE();
    case 394:
      ACCEPT_TOKEN(anon_sym_apply);
      END_STATE();
    case 395:
      if (lookahead == 't') ADVANCE(520);
      END_STATE();
    case 396:
      if (lookahead == '_') ADVANCE(521);
      END_STATE();
    case 397:
      if (lookahead == 'i') ADVANCE(522);
      END_STATE();
    case 398:
      if (lookahead == 't') ADVANCE(523);
      END_STATE();
    case 399:
      if (lookahead == 'p') ADVANCE(524);
      END_STATE();
    case 400:
      if (lookahead == 'a') ADVANCE(525);
      END_STATE();
    case 401:
      if (lookahead == 'r') ADVANCE(526);
      END_STATE();
    case 402:
      if (lookahead == 'c') ADVANCE(527);
      if (lookahead == 'i') ADVANCE(528);
      if (lookahead == 't') ADVANCE(529);
      END_STATE();
    case 403:
      ACCEPT_TOKEN(anon_sym_chain);
      END_STATE();
    case 404:
      if (lookahead == 'e') ADVANCE(530);
      END_STATE();
    case 405:
      if (lookahead == 'u') ADVANCE(531);
      END_STATE();
    case 406:
      if (lookahead == '-') ADVANCE(532);
      END_STATE();
    case 407:
      ACCEPT_TOKEN(anon_sym_click);
      END_STATE();
    case 408:
      ACCEPT_TOKEN(anon_sym_clock);
      END_STATE();
    case 409:
      if (lookahead == 't') ADVANCE(533);
      END_STATE();
    case 410:
      if (lookahead == 'c') ADVANCE(534);
      END_STATE();
    case 411:
      if (lookahead == 'i') ADVANCE(535);
      END_STATE();
    case 412:
      if (lookahead == '-') ADVANCE(536);
      END_STATE();
    case 413:
      if (lookahead == 'r') ADVANCE(537);
      END_STATE();
    case 414:
      ACCEPT_TOKEN(anon_sym_cycle);
      END_STATE();
    case 415:
      if (lookahead == 'e') ADVANCE(538);
      END_STATE();
    case 416:
      if (lookahead == 'r') ADVANCE(539);
      END_STATE();
    case 417:
      if (lookahead == 'r') ADVANCE(540);
      END_STATE();
    case 418:
      ACCEPT_TOKEN(anon_sym_drive);
      END_STATE();
    case 419:
      if (lookahead == 'b') ADVANCE(541);
      END_STATE();
    case 420:
      if (lookahead == 't') ADVANCE(542);
      END_STATE();
    case 421:
      if (lookahead == 'e') ADVANCE(543);
      END_STATE();
    case 422:
      if (lookahead == '_') ADVANCE(544);
      END_STATE();
    case 423:
      if (lookahead == 'l') ADVANCE(545);
      END_STATE();
    case 424:
      if (lookahead == 'i') ADVANCE(546);
      END_STATE();
    case 425:
      if (lookahead == 'r') ADVANCE(547);
      END_STATE();
    case 426:
      ACCEPT_TOKEN(anon_sym_flush);
      END_STATE();
    case 427:
      ACCEPT_TOKEN(anon_sym_given);
      END_STATE();
    case 428:
      if (lookahead == 'c') ADVANCE(548);
      END_STATE();
    case 429:
      ACCEPT_TOKEN(anon_sym_hover);
      END_STATE();
    case 430:
      if (lookahead == 't') ADVANCE(549);
      END_STATE();
    case 431:
      if (lookahead == 'd') ADVANCE(550);
      END_STATE();
    case 432:
      ACCEPT_TOKEN(anon_sym_input);
      END_STATE();
    case 433:
      if (lookahead == 'c') ADVANCE(551);
      END_STATE();
    case 434:
      ACCEPT_TOKEN(anon_sym_light);
      END_STATE();
    case 435:
      if (lookahead == 'e') ADVANCE(552);
      END_STATE();
    case 436:
      if (lookahead == 'i') ADVANCE(553);
      END_STATE();
    case 437:
      if (lookahead == 'p') ADVANCE(554);
      END_STATE();
    case 438:
      ACCEPT_TOKEN(anon_sym_macro);
      END_STATE();
    case 439:
      if (lookahead == 't') ADVANCE(555);
      END_STATE();
    case 440:
      ACCEPT_TOKEN(anon_sym_match);
      END_STATE();
    case 441:
      ACCEPT_TOKEN(anon_sym_media);
      END_STATE();
    case 442:
      if (lookahead == 'e') ADVANCE(556);
      END_STATE();
    case 443:
      if (lookahead == 'n') ADVANCE(557);
      if (lookahead == 'p') ADVANCE(558);
      if (lookahead == 'r') ADVANCE(559);
      END_STATE();
    case 444:
      ACCEPT_TOKEN(anon_sym_modal);
      END_STATE();
    case 445:
      ACCEPT_TOKEN(anon_sym_model);
      END_STATE();
    case 446:
      if (lookahead == '-') ADVANCE(560);
      END_STATE();
    case 447:
      ACCEPT_TOKEN(anon_sym_mount);
      END_STATE();
    case 448:
      ACCEPT_TOKEN(anon_sym_mouse);
      if (lookahead == '-') ADVANCE(561);
      END_STATE();
    case 449:
      if (lookahead == 'e') ADVANCE(562);
      END_STATE();
    case 450:
      if (lookahead == 'e') ADVANCE(563);
      END_STATE();
    case 451:
      if (lookahead == 'r') ADVANCE(564);
      END_STATE();
    case 452:
      if (lookahead == 'i') ADVANCE(565);
      END_STATE();
    case 453:
      if (lookahead == 'v') ADVANCE(566);
      END_STATE();
    case 454:
      if (lookahead == 'n') ADVANCE(567);
      END_STATE();
    case 455:
      if (lookahead == 'i') ADVANCE(568);
      END_STATE();
    case 456:
      if (lookahead == 'c') ADVANCE(569);
      END_STATE();
    case 457:
      if (lookahead == 'v') ADVANCE(570);
      END_STATE();
    case 458:
      if (lookahead == 't') ADVANCE(571);
      END_STATE();
    case 459:
      if (lookahead == 't') ADVANCE(572);
      END_STATE();
    case 460:
      if (lookahead == 's') ADVANCE(573);
      END_STATE();
    case 461:
      if (lookahead == 't') ADVANCE(574);
      END_STATE();
    case 462:
      if (lookahead == 'c') ADVANCE(575);
      END_STATE();
    case 463:
      if (lookahead == 's') ADVANCE(576);
      END_STATE();
    case 464:
      ACCEPT_TOKEN(anon_sym_pinch);
      END_STATE();
    case 465:
      if (lookahead == 'l') ADVANCE(577);
      END_STATE();
    case 466:
      if (lookahead == 'a') ADVANCE(578);
      END_STATE();
    case 467:
      if (lookahead == 'n') ADVANCE(579);
      if (lookahead == 't') ADVANCE(580);
      END_STATE();
    case 468:
      if (lookahead == 't') ADVANCE(581);
      END_STATE();
    case 469:
      ACCEPT_TOKEN(anon_sym_print);
      END_STATE();
    case 470:
      if (lookahead == 'a') ADVANCE(582);
      END_STATE();
    case 471:
      if (lookahead == 'r') ADVANCE(583);
      END_STATE();
    case 472:
      if (lookahead == 'r') ADVANCE(584);
      END_STATE();
    case 473:
      if (lookahead == '-') ADVANCE(585);
      END_STATE();
    case 474:
      if (lookahead == '-') ADVANCE(586);
      END_STATE();
    case 475:
      if (lookahead == 'i') ADVANCE(587);
      END_STATE();
    case 476:
      if (lookahead == 'd') ADVANCE(588);
      END_STATE();
    case 477:
      if (lookahead == 'e') ADVANCE(589);
      END_STATE();
    case 478:
      if (lookahead == 't') ADVANCE(590);
      END_STATE();
    case 479:
      if (lookahead == 'c') ADVANCE(591);
      END_STATE();
    case 480:
      if (lookahead == 'e') ADVANCE(592);
      END_STATE();
    case 481:
      if (lookahead == 'l') ADVANCE(593);
      END_STATE();
    case 482:
      if (lookahead == 'e') ADVANCE(594);
      END_STATE();
    case 483:
      if (lookahead == 'a') ADVANCE(595);
      END_STATE();
    case 484:
      ACCEPT_TOKEN(anon_sym_scene);
      END_STATE();
    case 485:
      if (lookahead == 't') ADVANCE(596);
      END_STATE();
    case 486:
      if (lookahead == 'l') ADVANCE(597);
      END_STATE();
    case 487:
      if (lookahead == 'n') ADVANCE(598);
      END_STATE();
    case 488:
      if (lookahead == 'r') ADVANCE(599);
      END_STATE();
    case 489:
      ACCEPT_TOKEN(anon_sym_share);
      END_STATE();
    case 490:
      ACCEPT_TOKEN(anon_sym_sheet);
      END_STATE();
    case 491:
      if (lookahead == 'n') ADVANCE(600);
      END_STATE();
    case 492:
      if (lookahead == 'h') ADVANCE(601);
      END_STATE();
    case 493:
      if (lookahead == 'h') ADVANCE(602);
      END_STATE();
    case 494:
      if (lookahead == 't') ADVANCE(603);
      END_STATE();
    case 495:
      ACCEPT_TOKEN(anon_sym_stack);
      END_STATE();
    case 496:
      ACCEPT_TOKEN(anon_sym_state);
      if (lookahead == '_') ADVANCE(604);
      END_STATE();
    case 497:
      if (lookahead == 'c') ADVANCE(605);
      END_STATE();
    case 498:
      ACCEPT_TOKEN(anon_sym_swarm);
      END_STATE();
    case 499:
      ACCEPT_TOKEN(anon_sym_swipe);
      END_STATE();
    case 500:
      if (lookahead == 'a') ADVANCE(606);
      END_STATE();
    case 501:
      if (lookahead == 'o') ADVANCE(607);
      if (lookahead == 's') ADVANCE(608);
      END_STATE();
    case 502:
      if (lookahead == 'l') ADVANCE(609);
      END_STATE();
    case 503:
      if (lookahead == 'i') ADVANCE(610);
      END_STATE();
    case 504:
      ACCEPT_TOKEN(anon_sym_toast);
      END_STATE();
    case 505:
      if (lookahead == '-') ADVANCE(611);
      END_STATE();
    case 506:
      if (lookahead == 'i') ADVANCE(612);
      END_STATE();
    case 507:
      if (lookahead == '-') ADVANCE(613);
      END_STATE();
    case 508:
      if (lookahead == '-') ADVANCE(614);
      END_STATE();
    case 509:
      if (lookahead == 'f') ADVANCE(615);
      END_STATE();
    case 510:
      if (lookahead == 'f') ADVANCE(616);
      if (lookahead == 'u') ADVANCE(617);
      END_STATE();
    case 511:
      if (lookahead == 'k') ADVANCE(618);
      END_STATE();
    case 512:
      if (lookahead == 'c') ADVANCE(619);
      END_STATE();
    case 513:
      if (lookahead == 'r') ADVANCE(620);
      END_STATE();
    case 514:
      ACCEPT_TOKEN(anon_sym_wload);
      END_STATE();
    case 515:
      ACCEPT_TOKEN(anon_sym_wloop);
      END_STATE();
    case 516:
      ACCEPT_TOKEN(anon_sym_ws_DASHon);
      END_STATE();
    case 517:
      if (lookahead == 'l') ADVANCE(621);
      END_STATE();
    case 518:
      if (lookahead == 'b') ADVANCE(622);
      END_STATE();
    case 519:
      if (lookahead == 'd') ADVANCE(623);
      END_STATE();
    case 520:
      ACCEPT_TOKEN(anon_sym_assert);
      if (lookahead == '-') ADVANCE(624);
      END_STATE();
    case 521:
      if (lookahead == 't') ADVANCE(625);
      END_STATE();
    case 522:
      if (lookahead == 'o') ADVANCE(626);
      END_STATE();
    case 523:
      if (lookahead == 'r') ADVANCE(627);
      END_STATE();
    case 524:
      if (lookahead == 'o') ADVANCE(628);
      END_STATE();
    case 525:
      ACCEPT_TOKEN(anon_sym_camera);
      if (lookahead == '-') ADVANCE(629);
      END_STATE();
    case 526:
      if (lookahead == 'e') ADVANCE(630);
      END_STATE();
    case 527:
      if (lookahead == 'o') ADVANCE(631);
      END_STATE();
    case 528:
      if (lookahead == 't') ADVANCE(632);
      END_STATE();
    case 529:
      if (lookahead == 'o') ADVANCE(633);
      END_STATE();
    case 530:
      if (lookahead == 'l') ADVANCE(634);
      END_STATE();
    case 531:
      if (lookahead == 'p') ADVANCE(635);
      END_STATE();
    case 532:
      if (lookahead == 'm') ADVANCE(636);
      END_STATE();
    case 533:
      if (lookahead == 'e') ADVANCE(637);
      END_STATE();
    case 534:
      if (lookahead == 't') ADVANCE(638);
      END_STATE();
    case 535:
      if (lookahead == 'n') ADVANCE(639);
      END_STATE();
    case 536:
      if (lookahead == 'b') ADVANCE(640);
      END_STATE();
    case 537:
      ACCEPT_TOKEN(anon_sym_cursor);
      END_STATE();
    case 538:
      ACCEPT_TOKEN(anon_sym_device);
      if (lookahead == '-') ADVANCE(641);
      END_STATE();
    case 539:
      if (lookahead == 't') ADVANCE(642);
      END_STATE();
    case 540:
      ACCEPT_TOKEN(anon_sym_drawer);
      END_STATE();
    case 541:
      if (lookahead == 'l') ADVANCE(643);
      END_STATE();
    case 542:
      ACCEPT_TOKEN(anon_sym_effect);
      END_STATE();
    case 543:
      if (lookahead == '_') ADVANCE(644);
      END_STATE();
    case 544:
      if (lookahead == 'l') ADVANCE(645);
      END_STATE();
    case 545:
      if (lookahead == 'e') ADVANCE(646);
      END_STATE();
    case 546:
      if (lookahead == 'n') ADVANCE(647);
      END_STATE();
    case 547:
      if (lookahead == 'e') ADVANCE(648);
      END_STATE();
    case 548:
      ACCEPT_TOKEN(anon_sym_haptic);
      END_STATE();
    case 549:
      ACCEPT_TOKEN(anon_sym_import);
      END_STATE();
    case 550:
      if (lookahead == 'e') ADVANCE(649);
      END_STATE();
    case 551:
      if (lookahead == 'a') ADVANCE(650);
      END_STATE();
    case 552:
      ACCEPT_TOKEN(anon_sym_locale);
      END_STATE();
    case 553:
      if (lookahead == 'o') ADVANCE(651);
      END_STATE();
    case 554:
      if (lookahead == 'r') ADVANCE(652);
      END_STATE();
    case 555:
      if (lookahead == 'i') ADVANCE(653);
      END_STATE();
    case 556:
      ACCEPT_TOKEN(anon_sym_mobile);
      if (lookahead == '-') ADVANCE(654);
      END_STATE();
    case 557:
      if (lookahead == 'a') ADVANCE(655);
      END_STATE();
    case 558:
      if (lookahead == 'e') ADVANCE(656);
      END_STATE();
    case 559:
      if (lookahead == 'e') ADVANCE(657);
      END_STATE();
    case 560:
      if (lookahead == 't') ADVANCE(658);
      END_STATE();
    case 561:
      if (lookahead == 'p') ADVANCE(659);
      END_STATE();
    case 562:
      ACCEPT_TOKEN(anon_sym_mutate);
      END_STATE();
    case 563:
      ACCEPT_TOKEN(anon_sym_native);
      END_STATE();
    case 564:
      if (lookahead == 'k') ADVANCE(660);
      END_STATE();
    case 565:
      if (lookahead == 'c') ADVANCE(661);
      END_STATE();
    case 566:
      if (lookahead == 'e') ADVANCE(662);
      END_STATE();
    case 567:
      if (lookahead == 'e') ADVANCE(663);
      END_STATE();
    case 568:
      if (lookahead == 'c') ADVANCE(664);
      END_STATE();
    case 569:
      if (lookahead == 'u') ADVANCE(665);
      END_STATE();
    case 570:
      if (lookahead == 'e') ADVANCE(666);
      END_STATE();
    case 571:
      if (lookahead == 'e') ADVANCE(667);
      END_STATE();
    case 572:
      if (lookahead == 'a') ADVANCE(668);
      END_STATE();
    case 573:
      if (lookahead == 'i') ADVANCE(669);
      END_STATE();
    case 574:
      ACCEPT_TOKEN(anon_sym_output);
      if (lookahead == '-') ADVANCE(670);
      END_STATE();
    case 575:
      if (lookahead == 'l') ADVANCE(671);
      END_STATE();
    case 576:
      if (lookahead == 't') ADVANCE(672);
      END_STATE();
    case 577:
      ACCEPT_TOKEN(anon_sym_portal);
      END_STATE();
    case 578:
      if (lookahead == 'i') ADVANCE(673);
      END_STATE();
    case 579:
      if (lookahead == 'c') ADVANCE(674);
      END_STATE();
    case 580:
      ACCEPT_TOKEN(anon_sym_preset);
      END_STATE();
    case 581:
      if (lookahead == 'i') ADVANCE(675);
      END_STATE();
    case 582:
      if (lookahead == 'm') ADVANCE(676);
      END_STATE();
    case 583:
      if (lookahead == 't') ADVANCE(677);
      END_STATE();
    case 584:
      if (lookahead == 'e') ADVANCE(678);
      END_STATE();
    case 585:
      if (lookahead == 'f') ADVANCE(679);
      if (lookahead == 'i') ADVANCE(680);
      if (lookahead == 'r') ADVANCE(681);
      END_STATE();
    case 586:
      if (lookahead == 't') ADVANCE(682);
      END_STATE();
    case 587:
      if (lookahead == 'm') ADVANCE(683);
      END_STATE();
    case 588:
      if (lookahead == '-') ADVANCE(684);
      END_STATE();
    case 589:
      if (lookahead == 'd') ADVANCE(685);
      END_STATE();
    case 590:
      ACCEPT_TOKEN(anon_sym_repeat);
      END_STATE();
    case 591:
      if (lookahead == 'e') ADVANCE(686);
      END_STATE();
    case 592:
      ACCEPT_TOKEN(anon_sym_resize);
      END_STATE();
    case 593:
      ACCEPT_TOKEN(anon_sym_reveal);
      END_STATE();
    case 594:
      if (lookahead == 'x') ADVANCE(687);
      END_STATE();
    case 595:
      if (lookahead == 'r') ADVANCE(688);
      END_STATE();
    case 596:
      ACCEPT_TOKEN(anon_sym_script);
      END_STATE();
    case 597:
      ACCEPT_TOKEN(anon_sym_scroll);
      if (lookahead == '-') ADVANCE(689);
      END_STATE();
    case 598:
      if (lookahead == 'c') ADVANCE(690);
      END_STATE();
    case 599:
      ACCEPT_TOKEN(anon_sym_shader);
      END_STATE();
    case 600:
      if (lookahead == 'e') ADVANCE(691);
      END_STATE();
    case 601:
      if (lookahead == '-') ADVANCE(692);
      END_STATE();
    case 602:
      if (lookahead == 'o') ADVANCE(693);
      END_STATE();
    case 603:
      ACCEPT_TOKEN(anon_sym_socket);
      END_STATE();
    case 604:
      if (lookahead == 'm') ADVANCE(694);
      END_STATE();
    case 605:
      if (lookahead == 'e') ADVANCE(695);
      END_STATE();
    case 606:
      if (lookahead == 't') ADVANCE(696);
      END_STATE();
    case 607:
      if (lookahead == 'n') ADVANCE(697);
      END_STATE();
    case 608:
      if (lookahead == 'k') ADVANCE(698);
      END_STATE();
    case 609:
      if (lookahead == 'i') ADVANCE(699);
      END_STATE();
    case 610:
      if (lookahead == 'n') ADVANCE(700);
      END_STATE();
    case 611:
      if (lookahead == 'd') ADVANCE(701);
      if (lookahead == 'l') ADVANCE(702);
      if (lookahead == 'p') ADVANCE(703);
      if (lookahead == 's') ADVANCE(704);
      if (lookahead == 't') ADVANCE(705);
      END_STATE();
    case 612:
      if (lookahead == 't') ADVANCE(706);
      END_STATE();
    case 613:
      if (lookahead == 'c') ADVANCE(707);
      END_STATE();
    case 614:
      if (lookahead == 'v') ADVANCE(708);
      END_STATE();
    case 615:
      if (lookahead == 'o') ADVANCE(709);
      END_STATE();
    case 616:
      if (lookahead == 'o') ADVANCE(710);
      END_STATE();
    case 617:
      if (lookahead == 'n') ADVANCE(711);
      END_STATE();
    case 618:
      ACCEPT_TOKEN(anon_sym_wclick);
      END_STATE();
    case 619:
      if (lookahead == 'k') ADVANCE(712);
      END_STATE();
    case 620:
      ACCEPT_TOKEN(anon_sym_whover);
      END_STATE();
    case 621:
      if (lookahead == 'l') ADVANCE(713);
      END_STATE();
    case 622:
      if (lookahead == 'l') ADVANCE(714);
      END_STATE();
    case 623:
      ACCEPT_TOKEN(anon_sym_android);
      END_STATE();
    case 624:
      if (lookahead == 'm') ADVANCE(715);
      END_STATE();
    case 625:
      if (lookahead == 'r') ADVANCE(716);
      END_STATE();
    case 626:
      if (lookahead == 'r') ADVANCE(717);
      END_STATE();
    case 627:
      if (lookahead == 'i') ADVANCE(718);
      END_STATE();
    case 628:
      if (lookahead == 'i') ADVANCE(719);
      END_STATE();
    case 629:
      if (lookahead == 's') ADVANCE(720);
      END_STATE();
    case 630:
      ACCEPT_TOKEN(anon_sym_capture);
      if (lookahead == 'T') ADVANCE(721);
      if (lookahead == '_') ADVANCE(722);
      END_STATE();
    case 631:
      if (lookahead == 'u') ADVANCE(723);
      END_STATE();
    case 632:
      if (lookahead == 'e') ADVANCE(724);
      END_STATE();
    case 633:
      if (lookahead == 't') ADVANCE(725);
      END_STATE();
    case 634:
      ACCEPT_TOKEN(anon_sym_channel);
      END_STATE();
    case 635:
      ACCEPT_TOKEN(anon_sym_cleanup);
      END_STATE();
    case 636:
      if (lookahead == 'o') ADVANCE(726);
      END_STATE();
    case 637:
      ACCEPT_TOKEN(anon_sym_compute);
      END_STATE();
    case 638:
      ACCEPT_TOKEN(anon_sym_connect);
      END_STATE();
    case 639:
      if (lookahead == 'e') ADVANCE(727);
      END_STATE();
    case 640:
      if (lookahead == 'e') ADVANCE(728);
      END_STATE();
    case 641:
      if (lookahead == 'c') ADVANCE(729);
      if (lookahead == 'f') ADVANCE(730);
      END_STATE();
    case 642:
      ACCEPT_TOKEN(anon_sym_distort);
      END_STATE();
    case 643:
      if (lookahead == 'e') ADVANCE(731);
      END_STATE();
    case 644:
      if (lookahead == 's') ADVANCE(732);
      if (lookahead == 't') ADVANCE(733);
      END_STATE();
    case 645:
      if (lookahead == 'i') ADVANCE(734);
      END_STATE();
    case 646:
      ACCEPT_TOKEN(anon_sym_example);
      END_STATE();
    case 647:
      ACCEPT_TOKEN(anon_sym_fade_DASHin);
      if (lookahead == '-') ADVANCE(735);
      END_STATE();
    case 648:
      ACCEPT_TOKEN(anon_sym_fixture);
      END_STATE();
    case 649:
      ACCEPT_TOKEN(anon_sym_include);
      END_STATE();
    case 650:
      if (lookahead == 'p') ADVANCE(736);
      END_STATE();
    case 651:
      if (lookahead == 'n') ADVANCE(737);
      END_STATE();
    case 652:
      if (lookahead == 'e') ADVANCE(738);
      END_STATE();
    case 653:
      if (lookahead == 'c') ADVANCE(739);
      END_STATE();
    case 654:
      if (lookahead == 'b') ADVANCE(740);
      if (lookahead == 'c') ADVANCE(741);
      if (lookahead == 'h') ADVANCE(742);
      if (lookahead == 'i') ADVANCE(743);
      if (lookahead == 'm') ADVANCE(744);
      if (lookahead == 's') ADVANCE(745);
      if (lookahead == 't') ADVANCE(746);
      END_STATE();
    case 655:
      if (lookahead == 't') ADVANCE(747);
      END_STATE();
    case 656:
      if (lookahead == 'r') ADVANCE(748);
      END_STATE();
    case 657:
      if (lookahead == 's') ADVANCE(749);
      END_STATE();
    case 658:
      if (lookahead == 'o') ADVANCE(750);
      END_STATE();
    case 659:
      if (lookahead == 'o') ADVANCE(751);
      END_STATE();
    case 660:
      ACCEPT_TOKEN(anon_sym_network);
      if (lookahead == '-') ADVANCE(752);
      END_STATE();
    case 661:
      if (lookahead == 'a') ADVANCE(753);
      END_STATE();
    case 662:
      ACCEPT_TOKEN(anon_sym_observe);
      END_STATE();
    case 663:
      ACCEPT_TOKEN(anon_sym_offline);
      END_STATE();
    case 664:
      if (lookahead == 'k') ADVANCE(754);
      END_STATE();
    case 665:
      if (lookahead == 's') ADVANCE(755);
      END_STATE();
    case 666:
      if (lookahead == 'r') ADVANCE(756);
      END_STATE();
    case 667:
      if (lookahead == 'r') ADVANCE(757);
      END_STATE();
    case 668:
      if (lookahead == 't') ADVANCE(758);
      END_STATE();
    case 669:
      if (lookahead == 'b') ADVANCE(759);
      END_STATE();
    case 670:
      if (lookahead == 'j') ADVANCE(760);
      END_STATE();
    case 671:
      if (lookahead == 'e') ADVANCE(761);
      END_STATE();
    case 672:
      ACCEPT_TOKEN(anon_sym_persist);
      END_STATE();
    case 673:
      if (lookahead == 't') ADVANCE(762);
      END_STATE();
    case 674:
      if (lookahead == 'e') ADVANCE(763);
      END_STATE();
    case 675:
      if (lookahead == 'v') ADVANCE(764);
      END_STATE();
    case 676:
      ACCEPT_TOKEN(anon_sym_program);
      END_STATE();
    case 677:
      if (lookahead == 'y') ADVANCE(765);
      END_STATE();
    case 678:
      if (lookahead == 'f') ADVANCE(766);
      END_STATE();
    case 679:
      if (lookahead == 'a') ADVANCE(767);
      END_STATE();
    case 680:
      if (lookahead == 'n') ADVANCE(768);
      END_STATE();
    case 681:
      if (lookahead == 'a') ADVANCE(769);
      END_STATE();
    case 682:
      if (lookahead == 'o') ADVANCE(770);
      END_STATE();
    case 683:
      if (lookahead == 'e') ADVANCE(771);
      END_STATE();
    case 684:
      if (lookahead == 't') ADVANCE(772);
      END_STATE();
    case 685:
      if (lookahead == '-') ADVANCE(773);
      END_STATE();
    case 686:
      ACCEPT_TOKEN(anon_sym_replace);
      if (lookahead == '-') ADVANCE(774);
      END_STATE();
    case 687:
      if (lookahead == 't') ADVANCE(775);
      END_STATE();
    case 688:
      if (lookahead == 'e') ADVANCE(776);
      END_STATE();
    case 689:
      if (lookahead == 's') ADVANCE(777);
      if (lookahead == 'v') ADVANCE(778);
      END_STATE();
    case 690:
      if (lookahead == 'e') ADVANCE(779);
      END_STATE();
    case 691:
      if (lookahead == 't') ADVANCE(780);
      END_STATE();
    case 692:
      if (lookahead == 's') ADVANCE(781);
      END_STATE();
    case 693:
      if (lookahead == 't') ADVANCE(782);
      END_STATE();
    case 694:
      if (lookahead == 'a') ADVANCE(783);
      END_STATE();
    case 695:
      ACCEPT_TOKEN(anon_sym_surface);
      END_STATE();
    case 696:
      if (lookahead == 'e') ADVANCE(784);
      END_STATE();
    case 697:
      if (lookahead == 'l') ADVANCE(785);
      END_STATE();
    case 698:
      if (lookahead == 'i') ADVANCE(786);
      END_STATE();
    case 699:
      if (lookahead == 's') ADVANCE(787);
      END_STATE();
    case 700:
      if (lookahead == 'e') ADVANCE(788);
      END_STATE();
    case 701:
      if (lookahead == 'r') ADVANCE(789);
      END_STATE();
    case 702:
      if (lookahead == 'o') ADVANCE(790);
      END_STATE();
    case 703:
      if (lookahead == 'i') ADVANCE(791);
      if (lookahead == 'r') ADVANCE(792);
      END_STATE();
    case 704:
      if (lookahead == 't') ADVANCE(793);
      if (lookahead == 'w') ADVANCE(794);
      END_STATE();
    case 705:
      if (lookahead == 'a') ADVANCE(795);
      END_STATE();
    case 706:
      if (lookahead == 'i') ADVANCE(796);
      END_STATE();
    case 707:
      if (lookahead == 'h') ADVANCE(797);
      END_STATE();
    case 708:
      if (lookahead == 'i') ADVANCE(798);
      END_STATE();
    case 709:
      if (lookahead == 'r') ADVANCE(799);
      END_STATE();
    case 710:
      if (lookahead == 'r') ADVANCE(800);
      END_STATE();
    case 711:
      if (lookahead == 't') ADVANCE(801);
      END_STATE();
    case 712:
      if (lookahead == 'e') ADVANCE(802);
      END_STATE();
    case 713:
      ACCEPT_TOKEN(anon_sym_wscroll);
      END_STATE();
    case 714:
      if (lookahead == 'e') ADVANCE(803);
      END_STATE();
    case 715:
      if (lookahead == 'o') ADVANCE(804);
      END_STATE();
    case 716:
      if (lookahead == 'a') ADVANCE(805);
      END_STATE();
    case 717:
      ACCEPT_TOKEN(anon_sym_behavior);
      END_STATE();
    case 718:
      if (lookahead == 'c') ADVANCE(806);
      END_STATE();
    case 719:
      if (lookahead == 'n') ADVANCE(807);
      END_STATE();
    case 720:
      if (lookahead == 'c') ADVANCE(808);
      END_STATE();
    case 721:
      if (lookahead == 'y') ADVANCE(809);
      END_STATE();
    case 722:
      if (lookahead == 's') ADVANCE(810);
      if (lookahead == 't') ADVANCE(811);
      END_STATE();
    case 723:
      if (lookahead == 'n') ADVANCE(812);
      END_STATE();
    case 724:
      if (lookahead == 'm') ADVANCE(813);
      END_STATE();
    case 725:
      if (lookahead == 'a') ADVANCE(814);
      END_STATE();
    case 726:
      if (lookahead == 'c') ADVANCE(815);
      END_STATE();
    case 727:
      if (lookahead == 'r') ADVANCE(816);
      END_STATE();
    case 728:
      if (lookahead == 'z') ADVANCE(817);
      END_STATE();
    case 729:
      if (lookahead == 'u') ADVANCE(818);
      END_STATE();
    case 730:
      if (lookahead == 'r') ADVANCE(819);
      END_STATE();
    case 731:
      ACCEPT_TOKEN(anon_sym_editable);
      if (lookahead == '-') ADVANCE(820);
      END_STATE();
    case 732:
      if (lookahead == 'i') ADVANCE(821);
      if (lookahead == 't') ADVANCE(822);
      END_STATE();
    case 733:
      if (lookahead == 'i') ADVANCE(823);
      END_STATE();
    case 734:
      if (lookahead == 's') ADVANCE(824);
      END_STATE();
    case 735:
      if (lookahead == 'd') ADVANCE(825);
      if (lookahead == 's') ADVANCE(826);
      if (lookahead == 'u') ADVANCE(827);
      END_STATE();
    case 736:
      if (lookahead == 'e') ADVANCE(828);
      END_STATE();
    case 737:
      ACCEPT_TOKEN(anon_sym_location);
      END_STATE();
    case 738:
      if (lookahead == 's') ADVANCE(829);
      END_STATE();
    case 739:
      ACCEPT_TOKEN(anon_sym_magnetic);
      if (lookahead == '-') ADVANCE(830);
      END_STATE();
    case 740:
      if (lookahead == 'u') ADVANCE(831);
      END_STATE();
    case 741:
      if (lookahead == 'o') ADVANCE(832);
      END_STATE();
    case 742:
      if (lookahead == 'e') ADVANCE(833);
      END_STATE();
    case 743:
      if (lookahead == 'n') ADVANCE(834);
      END_STATE();
    case 744:
      if (lookahead == 'o') ADVANCE(835);
      END_STATE();
    case 745:
      if (lookahead == 'e') ADVANCE(836);
      if (lookahead == 'w') ADVANCE(837);
      END_STATE();
    case 746:
      if (lookahead == 'e') ADVANCE(838);
      if (lookahead == 'h') ADVANCE(839);
      if (lookahead == 'o') ADVANCE(840);
      if (lookahead == 'y') ADVANCE(841);
      END_STATE();
    case 747:
      if (lookahead == 'i') ADVANCE(842);
      END_STATE();
    case 748:
      if (lookahead == 'm') ADVANCE(843);
      END_STATE();
    case 749:
      if (lookahead == 'p') ADVANCE(844);
      END_STATE();
    case 750:
      ACCEPT_TOKEN(anon_sym_morph_DASHto);
      if (lookahead == '-') ADVANCE(845);
      END_STATE();
    case 751:
      if (lookahead == 's') ADVANCE(846);
      END_STATE();
    case 752:
      if (lookahead == 's') ADVANCE(847);
      END_STATE();
    case 753:
      if (lookahead == 't') ADVANCE(848);
      END_STATE();
    case 754:
      ACCEPT_TOKEN(anon_sym_on_DASHclick);
      END_STATE();
    case 755:
      ACCEPT_TOKEN(anon_sym_on_DASHfocus);
      END_STATE();
    case 756:
      ACCEPT_TOKEN(anon_sym_on_DASHhover);
      END_STATE();
    case 757:
      if (lookahead == 's') ADVANCE(849);
      END_STATE();
    case 758:
      if (lookahead == 'i') ADVANCE(850);
      END_STATE();
    case 759:
      if (lookahead == 'l') ADVANCE(851);
      END_STATE();
    case 760:
      if (lookahead == 's') ADVANCE(852);
      END_STATE();
    case 761:
      if (lookahead == '-') ADVANCE(853);
      END_STATE();
    case 762:
      ACCEPT_TOKEN(anon_sym_portrait);
      END_STATE();
    case 763:
      ACCEPT_TOKEN(anon_sym_presence);
      END_STATE();
    case 764:
      if (lookahead == 'e') ADVANCE(854);
      END_STATE();
    case 765:
      ACCEPT_TOKEN(anon_sym_property);
      END_STATE();
    case 766:
      if (lookahead == 'r') ADVANCE(855);
      END_STATE();
    case 767:
      if (lookahead == 'l') ADVANCE(856);
      END_STATE();
    case 768:
      if (lookahead == 't') ADVANCE(857);
      END_STATE();
    case 769:
      if (lookahead == 'd') ADVANCE(858);
      END_STATE();
    case 770:
      if (lookahead == '-') ADVANCE(859);
      END_STATE();
    case 771:
      ACCEPT_TOKEN(anon_sym_realtime);
      END_STATE();
    case 772:
      if (lookahead == 'i') ADVANCE(860);
      END_STATE();
    case 773:
      if (lookahead == 'm') ADVANCE(861);
      END_STATE();
    case 774:
      if (lookahead == 'r') ADVANCE(862);
      END_STATE();
    case 775:
      ACCEPT_TOKEN(anon_sym_richtext);
      END_STATE();
    case 776:
      if (lookahead == 'a') ADVANCE(863);
      END_STATE();
    case 777:
      if (lookahead == 'p') ADVANCE(864);
      END_STATE();
    case 778:
      if (lookahead == 'i') ADVANCE(865);
      END_STATE();
    case 779:
      ACCEPT_TOKEN(anon_sym_sequence);
      END_STATE();
    case 780:
      if (lookahead == 'w') ADVANCE(866);
      END_STATE();
    case 781:
      if (lookahead == 'c') ADVANCE(867);
      END_STATE();
    case 782:
      ACCEPT_TOKEN(anon_sym_snapshot);
      END_STATE();
    case 783:
      if (lookahead == 'c') ADVANCE(868);
      END_STATE();
    case 784:
      ACCEPT_TOKEN(anon_sym_template);
      END_STATE();
    case 785:
      if (lookahead == 'y') ADVANCE(869);
      END_STATE();
    case 786:
      if (lookahead == 'p') ADVANCE(870);
      END_STATE();
    case 787:
      if (lookahead == 't') ADVANCE(871);
      END_STATE();
    case 788:
      ACCEPT_TOKEN(anon_sym_timeline);
      END_STATE();
    case 789:
      if (lookahead == 'a') ADVANCE(872);
      END_STATE();
    case 790:
      if (lookahead == 'n') ADVANCE(873);
      END_STATE();
    case 791:
      if (lookahead == 'n') ADVANCE(874);
      END_STATE();
    case 792:
      if (lookahead == 'e') ADVANCE(875);
      END_STATE();
    case 793:
      if (lookahead == 'a') ADVANCE(876);
      END_STATE();
    case 794:
      if (lookahead == 'i') ADVANCE(877);
      END_STATE();
    case 795:
      if (lookahead == 'p') ADVANCE(878);
      END_STATE();
    case 796:
      if (lookahead == 'o') ADVANCE(879);
      END_STATE();
    case 797:
      if (lookahead == 'a') ADVANCE(880);
      END_STATE();
    case 798:
      if (lookahead == 's') ADVANCE(881);
      END_STATE();
    case 799:
      ACCEPT_TOKEN(anon_sym_wait_DASHfor);
      END_STATE();
    case 800:
      if (lookahead == '_') ADVANCE(882);
      END_STATE();
    case 801:
      if (lookahead == 'i') ADVANCE(883);
      END_STATE();
    case 802:
      if (lookahead == 't') ADVANCE(884);
      END_STATE();
    case 803:
      ACCEPT_TOKEN(anon_sym_wvisible);
      END_STATE();
    case 804:
      if (lookahead == 'c') ADVANCE(885);
      END_STATE();
    case 805:
      if (lookahead == 'n') ADVANCE(886);
      END_STATE();
    case 806:
      ACCEPT_TOKEN(anon_sym_biometric);
      END_STATE();
    case 807:
      if (lookahead == 't') ADVANCE(887);
      END_STATE();
    case 808:
      if (lookahead == 'a') ADVANCE(888);
      END_STATE();
    case 809:
      if (lookahead == 'p') ADVANCE(889);
      END_STATE();
    case 810:
      if (lookahead == 'i') ADVANCE(890);
      if (lookahead == 't') ADVANCE(891);
      END_STATE();
    case 811:
      if (lookahead == 'i') ADVANCE(892);
      if (lookahead == 'y') ADVANCE(893);
      END_STATE();
    case 812:
      if (lookahead == 't') ADVANCE(894);
      END_STATE();
    case 813:
      if (lookahead == 's') ADVANCE(895);
      END_STATE();
    case 814:
      if (lookahead == 'l') ADVANCE(896);
      END_STATE();
    case 815:
      if (lookahead == 'k') ADVANCE(897);
      END_STATE();
    case 816:
      ACCEPT_TOKEN(anon_sym_container);
      END_STATE();
    case 817:
      if (lookahead == 'i') ADVANCE(898);
      END_STATE();
    case 818:
      if (lookahead == 's') ADVANCE(899);
      END_STATE();
    case 819:
      if (lookahead == 'a') ADVANCE(900);
      END_STATE();
    case 820:
      if (lookahead == 'b') ADVANCE(901);
      if (lookahead == 'm') ADVANCE(902);
      END_STATE();
    case 821:
      if (lookahead == 'g') ADVANCE(903);
      END_STATE();
    case 822:
      if (lookahead == 'a') ADVANCE(904);
      END_STATE();
    case 823:
      if (lookahead == 'm') ADVANCE(905);
      END_STATE();
    case 824:
      if (lookahead == 't') ADVANCE(906);
      END_STATE();
    case 825:
      if (lookahead == 'o') ADVANCE(907);
      END_STATE();
    case 826:
      if (lookahead == 't') ADVANCE(908);
      END_STATE();
    case 827:
      if (lookahead == 'p') ADVANCE(909);
      END_STATE();
    case 828:
      ACCEPT_TOKEN(anon_sym_landscape);
      END_STATE();
    case 829:
      if (lookahead == 's') ADVANCE(910);
      END_STATE();
    case 830:
      if (lookahead == 'g') ADVANCE(911);
      END_STATE();
    case 831:
      if (lookahead == 't') ADVANCE(912);
      END_STATE();
    case 832:
      if (lookahead == 'l') ADVANCE(913);
      END_STATE();
    case 833:
      if (lookahead == 'a') ADVANCE(914);
      END_STATE();
    case 834:
      if (lookahead == 'p') ADVANCE(915);
      END_STATE();
    case 835:
      if (lookahead == 't') ADVANCE(916);
      END_STATE();
    case 836:
      if (lookahead == 'a') ADVANCE(917);
      END_STATE();
    case 837:
      if (lookahead == 'i') ADVANCE(918);
      END_STATE();
    case 838:
      if (lookahead == 'x') ADVANCE(919);
      END_STATE();
    case 839:
      if (lookahead == 'e') ADVANCE(920);
      END_STATE();
    case 840:
      if (lookahead == 'k') ADVANCE(921);
      END_STATE();
    case 841:
      if (lookahead == 'p') ADVANCE(922);
      END_STATE();
    case 842:
      if (lookahead == 'v') ADVANCE(923);
      END_STATE();
    case 843:
      if (lookahead == 'i') ADVANCE(924);
      END_STATE();
    case 844:
      if (lookahead == 'o') ADVANCE(925);
      END_STATE();
    case 845:
      if (lookahead == 'c') ADVANCE(926);
      if (lookahead == 'f') ADVANCE(927);
      if (lookahead == 'g') ADVANCE(928);
      if (lookahead == 'i') ADVANCE(929);
      if (lookahead == 'r') ADVANCE(930);
      END_STATE();
    case 846:
      if (lookahead == 'i') ADVANCE(931);
      END_STATE();
    case 847:
      if (lookahead == 'e') ADVANCE(932);
      END_STATE();
    case 848:
      if (lookahead == 'i') ADVANCE(933);
      END_STATE();
    case 849:
      if (lookahead == 'e') ADVANCE(934);
      END_STATE();
    case 850:
      if (lookahead == 'o') ADVANCE(935);
      END_STATE();
    case 851:
      if (lookahead == 'e') ADVANCE(936);
      END_STATE();
    case 852:
      if (lookahead == 'o') ADVANCE(937);
      END_STATE();
    case 853:
      if (lookahead == 'f') ADVANCE(938);
      END_STATE();
    case 854:
      ACCEPT_TOKEN(anon_sym_primitive);
      END_STATE();
    case 855:
      if (lookahead == 'e') ADVANCE(939);
      END_STATE();
    case 856:
      if (lookahead == 'l') ADVANCE(940);
      END_STATE();
    case 857:
      if (lookahead == 'e') ADVANCE(941);
      END_STATE();
    case 858:
      if (lookahead == 'i') ADVANCE(942);
      END_STATE();
    case 859:
      if (lookahead == 'c') ADVANCE(943);
      if (lookahead == 'i') ADVANCE(944);
      if (lookahead == 'r') ADVANCE(945);
      END_STATE();
    case 860:
      if (lookahead == 'm') ADVANCE(946);
      END_STATE();
    case 861:
      if (lookahead == 'o') ADVANCE(947);
      END_STATE();
    case 862:
      if (lookahead == 'e') ADVANCE(948);
      END_STATE();
    case 863:
      ACCEPT_TOKEN(anon_sym_safe_DASHarea);
      if (lookahead == '-') ADVANCE(949);
      END_STATE();
    case 864:
      if (lookahead == 'y') ADVANCE(950);
      END_STATE();
    case 865:
      if (lookahead == 'e') ADVANCE(951);
      END_STATE();
    case 866:
      if (lookahead == 'o') ADVANCE(952);
      END_STATE();
    case 867:
      if (lookahead == 'r') ADVANCE(953);
      END_STATE();
    case 868:
      if (lookahead == 'h') ADVANCE(954);
      END_STATE();
    case 869:
      ACCEPT_TOKEN(anon_sym_test_DASHonly);
      END_STATE();
    case 870:
      ACCEPT_TOKEN(anon_sym_test_DASHskip);
      END_STATE();
    case 871:
      ACCEPT_TOKEN(anon_sym_test_list);
      END_STATE();
    case 872:
      if (lookahead == 'g') ADVANCE(955);
      END_STATE();
    case 873:
      if (lookahead == 'g') ADVANCE(956);
      END_STATE();
    case 874:
      if (lookahead == 'c') ADVANCE(957);
      END_STATE();
    case 875:
      if (lookahead == 's') ADVANCE(958);
      END_STATE();
    case 876:
      if (lookahead == 't') ADVANCE(959);
      END_STATE();
    case 877:
      if (lookahead == 'p') ADVANCE(960);
      END_STATE();
    case 878:
      ACCEPT_TOKEN(anon_sym_touch_DASHtap);
      END_STATE();
    case 879:
      if (lookahead == 'n') ADVANCE(961);
      END_STATE();
    case 880:
      if (lookahead == 'n') ADVANCE(962);
      END_STATE();
    case 881:
      if (lookahead == 'i') ADVANCE(963);
      END_STATE();
    case 882:
      if (lookahead == 's') ADVANCE(964);
      if (lookahead == 't') ADVANCE(965);
      END_STATE();
    case 883:
      if (lookahead == 'l') ADVANCE(966);
      END_STATE();
    case 884:
      ACCEPT_TOKEN(anon_sym_websocket);
      END_STATE();
    case 885:
      if (lookahead == 'k') ADVANCE(967);
      END_STATE();
    case 886:
      if (lookahead == 's') ADVANCE(968);
      END_STATE();
    case 887:
      ACCEPT_TOKEN(anon_sym_breakpoint);
      END_STATE();
    case 888:
      if (lookahead == 'n') ADVANCE(969);
      END_STATE();
    case 889:
      if (lookahead == 'e') ADVANCE(970);
      END_STATE();
    case 890:
      if (lookahead == 'g') ADVANCE(971);
      END_STATE();
    case 891:
      if (lookahead == 'a') ADVANCE(972);
      END_STATE();
    case 892:
      if (lookahead == 'm') ADVANCE(973);
      END_STATE();
    case 893:
      if (lookahead == 'p') ADVANCE(974);
      END_STATE();
    case 894:
      ACCEPT_TOKEN(anon_sym_cart_DASHcount);
      END_STATE();
    case 895:
      ACCEPT_TOKEN(anon_sym_cart_DASHitems);
      END_STATE();
    case 896:
      ACCEPT_TOKEN(anon_sym_cart_DASHtotal);
      END_STATE();
    case 897:
      if (lookahead == 's') ADVANCE(975);
      END_STATE();
    case 898:
      if (lookahead == 'e') ADVANCE(976);
      END_STATE();
    case 899:
      if (lookahead == 't') ADVANCE(977);
      END_STATE();
    case 900:
      if (lookahead == 'm') ADVANCE(978);
      END_STATE();
    case 901:
      if (lookahead == 'l') ADVANCE(979);
      END_STATE();
    case 902:
      if (lookahead == 'a') ADVANCE(980);
      END_STATE();
    case 903:
      if (lookahead == 'n') ADVANCE(981);
      END_STATE();
    case 904:
      if (lookahead == 't') ADVANCE(982);
      END_STATE();
    case 905:
      if (lookahead == 'i') ADVANCE(983);
      END_STATE();
    case 906:
      ACCEPT_TOKEN(anon_sym_error_list);
      END_STATE();
    case 907:
      if (lookahead == 'w') ADVANCE(984);
      END_STATE();
    case 908:
      if (lookahead == 'a') ADVANCE(985);
      END_STATE();
    case 909:
      ACCEPT_TOKEN(anon_sym_fade_DASHin_DASHup);
      END_STATE();
    case 910:
      ACCEPT_TOKEN(anon_sym_long_DASHpress);
      END_STATE();
    case 911:
      if (lookahead == 'l') ADVANCE(986);
      END_STATE();
    case 912:
      if (lookahead == 't') ADVANCE(987);
      END_STATE();
    case 913:
      if (lookahead == 'o') ADVANCE(988);
      END_STATE();
    case 914:
      if (lookahead == 'd') ADVANCE(989);
      END_STATE();
    case 915:
      if (lookahead == 'u') ADVANCE(990);
      END_STATE();
    case 916:
      if (lookahead == 'i') ADVANCE(991);
      END_STATE();
    case 917:
      if (lookahead == 'r') ADVANCE(992);
      END_STATE();
    case 918:
      if (lookahead == 'p') ADVANCE(993);
      END_STATE();
    case 919:
      if (lookahead == 't') ADVANCE(994);
      END_STATE();
    case 920:
      if (lookahead == 'm') ADVANCE(995);
      END_STATE();
    case 921:
      if (lookahead == 'e') ADVANCE(996);
      END_STATE();
    case 922:
      if (lookahead == 'o') ADVANCE(997);
      END_STATE();
    case 923:
      if (lookahead == 'e') ADVANCE(998);
      END_STATE();
    case 924:
      if (lookahead == 's') ADVANCE(999);
      END_STATE();
    case 925:
      if (lookahead == 'n') ADVANCE(1000);
      END_STATE();
    case 926:
      if (lookahead == 'l') ADVANCE(1001);
      if (lookahead == 'o') ADVANCE(1002);
      END_STATE();
    case 927:
      if (lookahead == 'a') ADVANCE(1003);
      END_STATE();
    case 928:
      if (lookahead == 'l') ADVANCE(1004);
      END_STATE();
    case 929:
      if (lookahead == 'n') ADVANCE(1005);
      END_STATE();
    case 930:
      if (lookahead == 'a') ADVANCE(1006);
      END_STATE();
    case 931:
      if (lookahead == 't') ADVANCE(1007);
      END_STATE();
    case 932:
      if (lookahead == 't') ADVANCE(1008);
      END_STATE();
    case 933:
      if (lookahead == 'o') ADVANCE(1009);
      END_STATE();
    case 934:
      if (lookahead == 'c') ADVANCE(1010);
      END_STATE();
    case 935:
      if (lookahead == 'n') ADVANCE(1011);
      END_STATE();
    case 936:
      ACCEPT_TOKEN(anon_sym_on_DASHvisible);
      END_STATE();
    case 937:
      if (lookahead == 'n') ADVANCE(1012);
      END_STATE();
    case 938:
      if (lookahead == 'i') ADVANCE(1013);
      END_STATE();
    case 939:
      if (lookahead == 's') ADVANCE(1014);
      END_STATE();
    case 940:
      if (lookahead == 'o') ADVANCE(1015);
      END_STATE();
    case 941:
      if (lookahead == 'n') ADVANCE(1016);
      END_STATE();
    case 942:
      if (lookahead == 'u') ADVANCE(1017);
      END_STATE();
    case 943:
      if (lookahead == 'e') ADVANCE(1018);
      END_STATE();
    case 944:
      if (lookahead == 'n') ADVANCE(1019);
      END_STATE();
    case 945:
      if (lookahead == 'a') ADVANCE(1020);
      END_STATE();
    case 946:
      if (lookahead == 'e') ADVANCE(1021);
      END_STATE();
    case 947:
      if (lookahead == 't') ADVANCE(1022);
      END_STATE();
    case 948:
      if (lookahead == 'g') ADVANCE(1023);
      END_STATE();
    case 949:
      if (lookahead == 'c') ADVANCE(1024);
      if (lookahead == 'i') ADVANCE(1025);
      END_STATE();
    case 950:
      ACCEPT_TOKEN(anon_sym_scroll_DASHspy);
      END_STATE();
    case 951:
      if (lookahead == 'w') ADVANCE(1026);
      END_STATE();
    case 952:
      if (lookahead == 'r') ADVANCE(1027);
      END_STATE();
    case 953:
      if (lookahead == 'o') ADVANCE(1028);
      END_STATE();
    case 954:
      if (lookahead == 'i') ADVANCE(1029);
      END_STATE();
    case 955:
      ACCEPT_TOKEN(anon_sym_touch_DASHdrag);
      END_STATE();
    case 956:
      if (lookahead == '-') ADVANCE(1030);
      END_STATE();
    case 957:
      if (lookahead == 'h') ADVANCE(1031);
      END_STATE();
    case 958:
      if (lookahead == 'e') ADVANCE(1032);
      END_STATE();
    case 959:
      if (lookahead == 'e') ADVANCE(1033);
      END_STATE();
    case 960:
      if (lookahead == 'e') ADVANCE(1034);
      END_STATE();
    case 961:
      ACCEPT_TOKEN(anon_sym_transition);
      END_STATE();
    case 962:
      if (lookahead == 'g') ADVANCE(1035);
      END_STATE();
    case 963:
      if (lookahead == 'b') ADVANCE(1036);
      END_STATE();
    case 964:
      if (lookahead == 'i') ADVANCE(1037);
      if (lookahead == 't') ADVANCE(1038);
      END_STATE();
    case 965:
      if (lookahead == 'i') ADVANCE(1039);
      END_STATE();
    case 966:
      ACCEPT_TOKEN(anon_sym_wait_until);
      END_STATE();
    case 967:
      if (lookahead == '-') ADVANCE(1040);
      END_STATE();
    case 968:
      if (lookahead == 'i') ADVANCE(1041);
      END_STATE();
    case 969:
      ACCEPT_TOKEN(anon_sym_camera_DASHscan);
      END_STATE();
    case 970:
      ACCEPT_TOKEN(anon_sym_captureType);
      END_STATE();
    case 971:
      if (lookahead == 'n') ADVANCE(1042);
      END_STATE();
    case 972:
      if (lookahead == 't') ADVANCE(1043);
      END_STATE();
    case 973:
      if (lookahead == 'i') ADVANCE(1044);
      END_STATE();
    case 974:
      if (lookahead == 'e') ADVANCE(1045);
      END_STATE();
    case 975:
      ACCEPT_TOKEN(anon_sym_clear_DASHmocks);
      END_STATE();
    case 976:
      if (lookahead == 'r') ADVANCE(1046);
      END_STATE();
    case 977:
      if (lookahead == 'o') ADVANCE(1047);
      END_STATE();
    case 978:
      if (lookahead == 'e') ADVANCE(1048);
      END_STATE();
    case 979:
      if (lookahead == 'o') ADVANCE(1049);
      END_STATE();
    case 980:
      if (lookahead == 'r') ADVANCE(1050);
      END_STATE();
    case 981:
      if (lookahead == 'a') ADVANCE(1051);
      END_STATE();
    case 982:
      if (lookahead == 'e') ADVANCE(1052);
      END_STATE();
    case 983:
      if (lookahead == 'n') ADVANCE(1053);
      END_STATE();
    case 984:
      if (lookahead == 'n') ADVANCE(1054);
      END_STATE();
    case 985:
      if (lookahead == 'g') ADVANCE(1055);
      END_STATE();
    case 986:
      if (lookahead == 'o') ADVANCE(1056);
      END_STATE();
    case 987:
      if (lookahead == 'o') ADVANCE(1057);
      END_STATE();
    case 988:
      if (lookahead == 'r') ADVANCE(1058);
      END_STATE();
    case 989:
      if (lookahead == 'i') ADVANCE(1059);
      END_STATE();
    case 990:
      if (lookahead == 't') ADVANCE(1060);
      END_STATE();
    case 991:
      if (lookahead == 'o') ADVANCE(1061);
      END_STATE();
    case 992:
      if (lookahead == 'c') ADVANCE(1062);
      END_STATE();
    case 993:
      if (lookahead == 'e') ADVANCE(1063);
      END_STATE();
    case 994:
      ACCEPT_TOKEN(anon_sym_mobile_DASHtext);
      if (lookahead == 'a') ADVANCE(1064);
      END_STATE();
    case 995:
      if (lookahead == 'e') ADVANCE(1065);
      END_STATE();
    case 996:
      if (lookahead == 'n') ADVANCE(1066);
      END_STATE();
    case 997:
      if (lookahead == 'g') ADVANCE(1067);
      END_STATE();
    case 998:
      ACCEPT_TOKEN(anon_sym_mock_DASHnative);
      END_STATE();
    case 999:
      if (lookahead == 's') ADVANCE(1068);
      END_STATE();
    case 1000:
      if (lookahead == 's') ADVANCE(1069);
      END_STATE();
    case 1001:
      if (lookahead == 'i') ADVANCE(1070);
      END_STATE();
    case 1002:
      if (lookahead == 'l') ADVANCE(1071);
      END_STATE();
    case 1003:
      if (lookahead == 'l') ADVANCE(1072);
      END_STATE();
    case 1004:
      if (lookahead == 'o') ADVANCE(1073);
      END_STATE();
    case 1005:
      if (lookahead == 't') ADVANCE(1074);
      END_STATE();
    case 1006:
      if (lookahead == 'd') ADVANCE(1075);
      END_STATE();
    case 1007:
      if (lookahead == 'i') ADVANCE(1076);
      END_STATE();
    case 1008:
      ACCEPT_TOKEN(anon_sym_network_DASHset);
      END_STATE();
    case 1009:
      if (lookahead == 'n') ADVANCE(1077);
      END_STATE();
    case 1010:
      if (lookahead == 't') ADVANCE(1078);
      END_STATE();
    case 1011:
      ACCEPT_TOKEN(anon_sym_on_DASHmutation);
      END_STATE();
    case 1012:
      ACCEPT_TOKEN(anon_sym_output_DASHjson);
      END_STATE();
    case 1013:
      if (lookahead == 'e') ADVANCE(1079);
      END_STATE();
    case 1014:
      if (lookahead == 'h') ADVANCE(1080);
      END_STATE();
    case 1015:
      if (lookahead == 'f') ADVANCE(1081);
      END_STATE();
    case 1016:
      if (lookahead == 's') ADVANCE(1082);
      END_STATE();
    case 1017:
      if (lookahead == 's') ADVANCE(1083);
      END_STATE();
    case 1018:
      if (lookahead == 'n') ADVANCE(1084);
      END_STATE();
    case 1019:
      if (lookahead == 't') ADVANCE(1085);
      END_STATE();
    case 1020:
      if (lookahead == 'd') ADVANCE(1086);
      END_STATE();
    case 1021:
      if (lookahead == 'l') ADVANCE(1087);
      END_STATE();
    case 1022:
      if (lookahead == 'i') ADVANCE(1088);
      END_STATE();
    case 1023:
      if (lookahead == 'e') ADVANCE(1089);
      END_STATE();
    case 1024:
      if (lookahead == 'o') ADVANCE(1090);
      END_STATE();
    case 1025:
      if (lookahead == 'n') ADVANCE(1091);
      END_STATE();
    case 1026:
      ACCEPT_TOKEN(anon_sym_scroll_DASHview);
      END_STATE();
    case 1027:
      if (lookahead == 'k') ADVANCE(1092);
      END_STATE();
    case 1028:
      if (lookahead == 'l') ADVANCE(1093);
      END_STATE();
    case 1029:
      if (lookahead == 'n') ADVANCE(1094);
      END_STATE();
    case 1030:
      if (lookahead == 'p') ADVANCE(1095);
      END_STATE();
    case 1031:
      ACCEPT_TOKEN(anon_sym_touch_DASHpinch);
      END_STATE();
    case 1032:
      if (lookahead == 't') ADVANCE(1096);
      END_STATE();
    case 1033:
      ACCEPT_TOKEN(anon_sym_touch_DASHstate);
      END_STATE();
    case 1034:
      ACCEPT_TOKEN(anon_sym_touch_DASHswipe);
      END_STATE();
    case 1035:
      if (lookahead == 'e') ADVANCE(1097);
      END_STATE();
    case 1036:
      if (lookahead == 'i') ADVANCE(1098);
      END_STATE();
    case 1037:
      if (lookahead == 'g') ADVANCE(1099);
      END_STATE();
    case 1038:
      if (lookahead == 'a') ADVANCE(1100);
      END_STATE();
    case 1039:
      if (lookahead == 'm') ADVANCE(1101);
      END_STATE();
    case 1040:
      if (lookahead == 'c') ADVANCE(1102);
      END_STATE();
    case 1041:
      if (lookahead == 't') ADVANCE(1103);
      END_STATE();
    case 1042:
      if (lookahead == 'a') ADVANCE(1104);
      END_STATE();
    case 1043:
      if (lookahead == 'e') ADVANCE(1105);
      END_STATE();
    case 1044:
      if (lookahead == 'n') ADVANCE(1106);
      END_STATE();
    case 1045:
      ACCEPT_TOKEN(anon_sym_capture_type);
      END_STATE();
    case 1046:
      ACCEPT_TOKEN(anon_sym_cubic_DASHbezier);
      END_STATE();
    case 1047:
      if (lookahead == 'm') ADVANCE(1107);
      END_STATE();
    case 1048:
      ACCEPT_TOKEN(anon_sym_device_DASHframe);
      END_STATE();
    case 1049:
      if (lookahead == 'c') ADVANCE(1108);
      END_STATE();
    case 1050:
      if (lookahead == 'k') ADVANCE(1109);
      END_STATE();
    case 1051:
      if (lookahead == 'l') ADVANCE(1110);
      END_STATE();
    case 1052:
      if (lookahead == '_') ADVANCE(1111);
      END_STATE();
    case 1053:
      if (lookahead == 'g') ADVANCE(1112);
      END_STATE();
    case 1054:
      ACCEPT_TOKEN(anon_sym_fade_DASHin_DASHdown);
      END_STATE();
    case 1055:
      if (lookahead == 'g') ADVANCE(1113);
      END_STATE();
    case 1056:
      if (lookahead == 'w') ADVANCE(1114);
      END_STATE();
    case 1057:
      if (lookahead == 'n') ADVANCE(1115);
      END_STATE();
    case 1058:
      if (lookahead == 's') ADVANCE(1116);
      END_STATE();
    case 1059:
      if (lookahead == 'n') ADVANCE(1117);
      END_STATE();
    case 1060:
      ACCEPT_TOKEN(anon_sym_mobile_DASHinput);
      if (lookahead == '-') ADVANCE(1118);
      END_STATE();
    case 1061:
      if (lookahead == 'n') ADVANCE(1119);
      END_STATE();
    case 1062:
      if (lookahead == 'h') ADVANCE(1120);
      END_STATE();
    case 1063:
      ACCEPT_TOKEN(anon_sym_mobile_DASHswipe);
      END_STATE();
    case 1064:
      if (lookahead == 'r') ADVANCE(1121);
      END_STATE();
    case 1065:
      ACCEPT_TOKEN(anon_sym_mobile_DASHtheme);
      END_STATE();
    case 1066:
      if (lookahead == 's') ADVANCE(1122);
      END_STATE();
    case 1067:
      if (lookahead == 'r') ADVANCE(1123);
      END_STATE();
    case 1068:
      if (lookahead == 'i') ADVANCE(1124);
      END_STATE();
    case 1069:
      if (lookahead == 'e') ADVANCE(1125);
      END_STATE();
    case 1070:
      if (lookahead == 'c') ADVANCE(1126);
      END_STATE();
    case 1071:
      if (lookahead == 'o') ADVANCE(1127);
      END_STATE();
    case 1072:
      if (lookahead == 'l') ADVANCE(1128);
      END_STATE();
    case 1073:
      if (lookahead == 'w') ADVANCE(1129);
      END_STATE();
    case 1074:
      if (lookahead == 'e') ADVANCE(1130);
      END_STATE();
    case 1075:
      if (lookahead == 'i') ADVANCE(1131);
      END_STATE();
    case 1076:
      if (lookahead == 'o') ADVANCE(1132);
      END_STATE();
    case 1077:
      ACCEPT_TOKEN(anon_sym_notification);
      END_STATE();
    case 1078:
      ACCEPT_TOKEN(anon_sym_on_DASHintersect);
      END_STATE();
    case 1079:
      if (lookahead == 'l') ADVANCE(1133);
      END_STATE();
    case 1080:
      ACCEPT_TOKEN(anon_sym_pull_DASHrefresh);
      END_STATE();
    case 1081:
      if (lookahead == 'f') ADVANCE(1134);
      END_STATE();
    case 1082:
      if (lookahead == 'i') ADVANCE(1135);
      END_STATE();
    case 1083:
      ACCEPT_TOKEN(anon_sym_pulse_DASHradius);
      END_STATE();
    case 1084:
      if (lookahead == 't') ADVANCE(1136);
      END_STATE();
    case 1085:
      if (lookahead == 'e') ADVANCE(1137);
      END_STATE();
    case 1086:
      if (lookahead == 'i') ADVANCE(1138);
      END_STATE();
    case 1087:
      if (lookahead == 'i') ADVANCE(1139);
      END_STATE();
    case 1088:
      if (lookahead == 'o') ADVANCE(1140);
      END_STATE();
    case 1089:
      if (lookahead == 'x') ADVANCE(1141);
      END_STATE();
    case 1090:
      if (lookahead == 'n') ADVANCE(1142);
      END_STATE();
    case 1091:
      if (lookahead == 's') ADVANCE(1143);
      END_STATE();
    case 1092:
      ACCEPT_TOKEN(anon_sym_slow_DASHnetwork);
      END_STATE();
    case 1093:
      if (lookahead == 'l') ADVANCE(1144);
      END_STATE();
    case 1094:
      if (lookahead == 'e') ADVANCE(1145);
      END_STATE();
    case 1095:
      if (lookahead == 'r') ADVANCE(1146);
      END_STATE();
    case 1096:
      ACCEPT_TOKEN(anon_sym_touch_DASHpreset);
      END_STATE();
    case 1097:
      ACCEPT_TOKEN(anon_sym_value_DASHchange);
      END_STATE();
    case 1098:
      if (lookahead == 'l') ADVANCE(1147);
      END_STATE();
    case 1099:
      if (lookahead == 'n') ADVANCE(1148);
      END_STATE();
    case 1100:
      if (lookahead == 't') ADVANCE(1149);
      END_STATE();
    case 1101:
      if (lookahead == 'e') ADVANCE(1150);
      END_STATE();
    case 1102:
      if (lookahead == 'a') ADVANCE(1151);
      END_STATE();
    case 1103:
      if (lookahead == 'i') ADVANCE(1152);
      END_STATE();
    case 1104:
      if (lookahead == 'l') ADVANCE(1153);
      END_STATE();
    case 1105:
      if (lookahead == '_') ADVANCE(1154);
      END_STATE();
    case 1106:
      if (lookahead == 'g') ADVANCE(1155);
      END_STATE();
    case 1107:
      ACCEPT_TOKEN(anon_sym_device_DASHcustom);
      END_STATE();
    case 1108:
      if (lookahead == 'k') ADVANCE(1156);
      END_STATE();
    case 1109:
      ACCEPT_TOKEN(anon_sym_editable_DASHmark);
      END_STATE();
    case 1110:
      if (lookahead == '_') ADVANCE(1157);
      END_STATE();
    case 1111:
      if (lookahead == 't') ADVANCE(1158);
      END_STATE();
    case 1112:
      ACCEPT_TOKEN(anon_sym_enable_timing);
      END_STATE();
    case 1113:
      if (lookahead == 'e') ADVANCE(1159);
      END_STATE();
    case 1114:
      ACCEPT_TOKEN(anon_sym_magnetic_DASHglow);
      END_STATE();
    case 1115:
      ACCEPT_TOKEN(anon_sym_mobile_DASHbutton);
      END_STATE();
    case 1116:
      ACCEPT_TOKEN(anon_sym_mobile_DASHcolors);
      if (lookahead == '-') ADVANCE(1160);
      END_STATE();
    case 1117:
      if (lookahead == 'g') ADVANCE(1161);
      END_STATE();
    case 1118:
      if (lookahead == 'f') ADVANCE(1162);
      END_STATE();
    case 1119:
      ACCEPT_TOKEN(anon_sym_mobile_DASHmotion);
      END_STATE();
    case 1120:
      ACCEPT_TOKEN(anon_sym_mobile_DASHsearch);
      END_STATE();
    case 1121:
      if (lookahead == 'e') ADVANCE(1163);
      END_STATE();
    case 1122:
      ACCEPT_TOKEN(anon_sym_mobile_DASHtokens);
      END_STATE();
    case 1123:
      if (lookahead == 'a') ADVANCE(1164);
      END_STATE();
    case 1124:
      if (lookahead == 'o') ADVANCE(1165);
      END_STATE();
    case 1125:
      ACCEPT_TOKEN(anon_sym_mock_DASHresponse);
      END_STATE();
    case 1126:
      if (lookahead == 'k') ADVANCE(1166);
      END_STATE();
    case 1127:
      if (lookahead == 'r') ADVANCE(1167);
      END_STATE();
    case 1128:
      if (lookahead == 'o') ADVANCE(1168);
      END_STATE();
    case 1129:
      ACCEPT_TOKEN(anon_sym_morph_DASHto_DASHglow);
      END_STATE();
    case 1130:
      if (lookahead == 'n') ADVANCE(1169);
      END_STATE();
    case 1131:
      if (lookahead == 'u') ADVANCE(1170);
      END_STATE();
    case 1132:
      if (lookahead == 'n') ADVANCE(1171);
      END_STATE();
    case 1133:
      if (lookahead == 'd') ADVANCE(1172);
      END_STATE();
    case 1134:
      ACCEPT_TOKEN(anon_sym_pulse_DASHfalloff);
      END_STATE();
    case 1135:
      if (lookahead == 't') ADVANCE(1173);
      END_STATE();
    case 1136:
      if (lookahead == 'e') ADVANCE(1174);
      END_STATE();
    case 1137:
      if (lookahead == 'n') ADVANCE(1175);
      END_STATE();
    case 1138:
      if (lookahead == 'u') ADVANCE(1176);
      END_STATE();
    case 1139:
      if (lookahead == 'n') ADVANCE(1177);
      END_STATE();
    case 1140:
      if (lookahead == 'n') ADVANCE(1178);
      END_STATE();
    case 1141:
      ACCEPT_TOKEN(anon_sym_replace_DASHregex);
      END_STATE();
    case 1142:
      if (lookahead == 't') ADVANCE(1179);
      END_STATE();
    case 1143:
      if (lookahead == 'e') ADVANCE(1180);
      END_STATE();
    case 1144:
      ACCEPT_TOKEN(anon_sym_smooth_DASHscroll);
      END_STATE();
    case 1145:
      ACCEPT_TOKEN(anon_sym_state_machine);
      if (lookahead == '_') ADVANCE(1181);
      END_STATE();
    case 1146:
      if (lookahead == 'e') ADVANCE(1182);
      END_STATE();
    case 1147:
      if (lookahead == 'i') ADVANCE(1183);
      END_STATE();
    case 1148:
      if (lookahead == 'a') ADVANCE(1184);
      END_STATE();
    case 1149:
      if (lookahead == 'e') ADVANCE(1185);
      END_STATE();
    case 1150:
      if (lookahead == 'l') ADVANCE(1186);
      END_STATE();
    case 1151:
      if (lookahead == 'l') ADVANCE(1187);
      END_STATE();
    case 1152:
      if (lookahead == 'o') ADVANCE(1188);
      END_STATE();
    case 1153:
      if (lookahead == '_') ADVANCE(1189);
      END_STATE();
    case 1154:
      if (lookahead == 'r') ADVANCE(1190);
      END_STATE();
    case 1155:
      if (lookahead == '_') ADVANCE(1191);
      END_STATE();
    case 1156:
      ACCEPT_TOKEN(anon_sym_editable_DASHblock);
      END_STATE();
    case 1157:
      if (lookahead == 't') ADVANCE(1192);
      END_STATE();
    case 1158:
      if (lookahead == 'r') ADVANCE(1193);
      END_STATE();
    case 1159:
      if (lookahead == 'r') ADVANCE(1194);
      END_STATE();
    case 1160:
      if (lookahead == 'd') ADVANCE(1195);
      END_STATE();
    case 1161:
      ACCEPT_TOKEN(anon_sym_mobile_DASHheading);
      END_STATE();
    case 1162:
      if (lookahead == 'i') ADVANCE(1196);
      END_STATE();
    case 1163:
      if (lookahead == 'a') ADVANCE(1197);
      END_STATE();
    case 1164:
      if (lookahead == 'p') ADVANCE(1198);
      END_STATE();
    case 1165:
      if (lookahead == 'n') ADVANCE(1199);
      END_STATE();
    case 1166:
      ACCEPT_TOKEN(anon_sym_morph_DASHto_DASHclick);
      END_STATE();
    case 1167:
      ACCEPT_TOKEN(anon_sym_morph_DASHto_DASHcolor);
      END_STATE();
    case 1168:
      if (lookahead == 'f') ADVANCE(1200);
      END_STATE();
    case 1169:
      if (lookahead == 's') ADVANCE(1201);
      END_STATE();
    case 1170:
      if (lookahead == 's') ADVANCE(1202);
      END_STATE();
    case 1171:
      ACCEPT_TOKEN(anon_sym_mouse_DASHposition);
      END_STATE();
    case 1172:
      ACCEPT_TOKEN(anon_sym_particle_DASHfield);
      END_STATE();
    case 1173:
      if (lookahead == 'y') ADVANCE(1203);
      END_STATE();
    case 1174:
      if (lookahead == 'r') ADVANCE(1204);
      END_STATE();
    case 1175:
      if (lookahead == 's') ADVANCE(1205);
      END_STATE();
    case 1176:
      if (lookahead == 's') ADVANCE(1206);
      END_STATE();
    case 1177:
      if (lookahead == 'e') ADVANCE(1207);
      END_STATE();
    case 1178:
      ACCEPT_TOKEN(anon_sym_reduced_DASHmotion);
      END_STATE();
    case 1179:
      if (lookahead == 'a') ADVANCE(1208);
      END_STATE();
    case 1180:
      if (lookahead == 't') ADVANCE(1209);
      END_STATE();
    case 1181:
      if (lookahead == 'b') ADVANCE(1210);
      END_STATE();
    case 1182:
      if (lookahead == 's') ADVANCE(1211);
      END_STATE();
    case 1183:
      if (lookahead == 't') ADVANCE(1212);
      END_STATE();
    case 1184:
      if (lookahead == 'l') ADVANCE(1213);
      END_STATE();
    case 1185:
      ACCEPT_TOKEN(anon_sym_wait_for_state);
      END_STATE();
    case 1186:
      if (lookahead == 'i') ADVANCE(1214);
      END_STATE();
    case 1187:
      if (lookahead == 'l') ADVANCE(1215);
      END_STATE();
    case 1188:
      if (lookahead == 'n') ADVANCE(1216);
      END_STATE();
    case 1189:
      if (lookahead == 'r') ADVANCE(1217);
      END_STATE();
    case 1190:
      if (lookahead == 'e') ADVANCE(1218);
      END_STATE();
    case 1191:
      if (lookahead == 'r') ADVANCE(1219);
      END_STATE();
    case 1192:
      if (lookahead == 'r') ADVANCE(1220);
      END_STATE();
    case 1193:
      if (lookahead == 'a') ADVANCE(1221);
      END_STATE();
    case 1194:
      ACCEPT_TOKEN(anon_sym_fade_DASHin_DASHstagger);
      END_STATE();
    case 1195:
      if (lookahead == 'a') ADVANCE(1222);
      END_STATE();
    case 1196:
      if (lookahead == 'e') ADVANCE(1223);
      END_STATE();
    case 1197:
      ACCEPT_TOKEN(anon_sym_mobile_DASHtextarea);
      END_STATE();
    case 1198:
      if (lookahead == 'h') ADVANCE(1224);
      END_STATE();
    case 1199:
      ACCEPT_TOKEN(anon_sym_mock_DASHpermission);
      END_STATE();
    case 1200:
      if (lookahead == 'f') ADVANCE(1225);
      END_STATE();
    case 1201:
      if (lookahead == 'i') ADVANCE(1226);
      END_STATE();
    case 1202:
      ACCEPT_TOKEN(anon_sym_morph_DASHto_DASHradius);
      END_STATE();
    case 1203:
      ACCEPT_TOKEN(anon_sym_pulse_DASHintensity);
      END_STATE();
    case 1204:
      ACCEPT_TOKEN(anon_sym_react_DASHto_DASHcenter);
      END_STATE();
    case 1205:
      if (lookahead == 'i') ADVANCE(1227);
      END_STATE();
    case 1206:
      ACCEPT_TOKEN(anon_sym_react_DASHto_DASHradius);
      END_STATE();
    case 1207:
      ACCEPT_TOKEN(anon_sym_record_DASHtimeline);
      END_STATE();
    case 1208:
      if (lookahead == 'i') ADVANCE(1228);
      END_STATE();
    case 1209:
      ACCEPT_TOKEN(anon_sym_safe_DASHarea_DASHinset);
      END_STATE();
    case 1210:
      if (lookahead == 'l') ADVANCE(1229);
      END_STATE();
    case 1211:
      if (lookahead == 's') ADVANCE(1230);
      END_STATE();
    case 1212:
      if (lookahead == 'y') ADVANCE(1231);
      END_STATE();
    case 1213:
      ACCEPT_TOKEN(anon_sym_wait_for_signal);
      END_STATE();
    case 1214:
      if (lookahead == 'n') ADVANCE(1232);
      END_STATE();
    case 1215:
      if (lookahead == 'e') ADVANCE(1233);
      END_STATE();
    case 1216:
      ACCEPT_TOKEN(anon_sym_async_transition);
      END_STATE();
    case 1217:
      if (lookahead == 'e') ADVANCE(1234);
      END_STATE();
    case 1218:
      if (lookahead == 'p') ADVANCE(1235);
      END_STATE();
    case 1219:
      if (lookahead == 'e') ADVANCE(1236);
      END_STATE();
    case 1220:
      if (lookahead == 'a') ADVANCE(1237);
      END_STATE();
    case 1221:
      if (lookahead == 'c') ADVANCE(1238);
      END_STATE();
    case 1222:
      if (lookahead == 'r') ADVANCE(1239);
      END_STATE();
    case 1223:
      if (lookahead == 'l') ADVANCE(1240);
      END_STATE();
    case 1224:
      if (lookahead == 'y') ADVANCE(1241);
      END_STATE();
    case 1225:
      ACCEPT_TOKEN(anon_sym_morph_DASHto_DASHfalloff);
      END_STATE();
    case 1226:
      if (lookahead == 't') ADVANCE(1242);
      END_STATE();
    case 1227:
      if (lookahead == 't') ADVANCE(1243);
      END_STATE();
    case 1228:
      if (lookahead == 'n') ADVANCE(1244);
      END_STATE();
    case 1229:
      if (lookahead == 'o') ADVANCE(1245);
      END_STATE();
    case 1230:
      ACCEPT_TOKEN(anon_sym_touch_DASHlong_DASHpress);
      END_STATE();
    case 1231:
      ACCEPT_TOKEN(anon_sym_video_DASHvisibility);
      END_STATE();
    case 1232:
      if (lookahead == 'e') ADVANCE(1246);
      END_STATE();
    case 1233:
      if (lookahead == 'd') ADVANCE(1247);
      END_STATE();
    case 1234:
      if (lookahead == 'p') ADVANCE(1248);
      END_STATE();
    case 1235:
      if (lookahead == 'o') ADVANCE(1249);
      END_STATE();
    case 1236:
      if (lookahead == 'p') ADVANCE(1250);
      END_STATE();
    case 1237:
      if (lookahead == 'c') ADVANCE(1251);
      END_STATE();
    case 1238:
      if (lookahead == 'k') ADVANCE(1252);
      END_STATE();
    case 1239:
      if (lookahead == 'k') ADVANCE(1253);
      END_STATE();
    case 1240:
      if (lookahead == 'd') ADVANCE(1254);
      END_STATE();
    case 1241:
      ACCEPT_TOKEN(anon_sym_mobile_DASHtypography);
      END_STATE();
    case 1242:
      if (lookahead == 'y') ADVANCE(1255);
      END_STATE();
    case 1243:
      if (lookahead == 'y') ADVANCE(1256);
      END_STATE();
    case 1244:
      if (lookahead == 'e') ADVANCE(1257);
      END_STATE();
    case 1245:
      if (lookahead == 'c') ADVANCE(1258);
      END_STATE();
    case 1246:
      ACCEPT_TOKEN(anon_sym_wait_for_timeline);
      END_STATE();
    case 1247:
      ACCEPT_TOKEN(anon_sym_assert_DASHmock_DASHcalled);
      END_STATE();
    case 1248:
      if (lookahead == 'o') ADVANCE(1259);
      END_STATE();
    case 1249:
      if (lookahead == 'r') ADVANCE(1260);
      END_STATE();
    case 1250:
      if (lookahead == 'o') ADVANCE(1261);
      END_STATE();
    case 1251:
      if (lookahead == 'k') ADVANCE(1262);
      END_STATE();
    case 1252:
      if (lookahead == 'i') ADVANCE(1263);
      END_STATE();
    case 1253:
      ACCEPT_TOKEN(anon_sym_mobile_DASHcolors_DASHdark);
      END_STATE();
    case 1254:
      ACCEPT_TOKEN(anon_sym_mobile_DASHinput_DASHfield);
      END_STATE();
    case 1255:
      ACCEPT_TOKEN(anon_sym_morph_DASHto_DASHintensity);
      END_STATE();
    case 1256:
      ACCEPT_TOKEN(anon_sym_react_DASHto_DASHintensity);
      END_STATE();
    case 1257:
      if (lookahead == 'r') ADVANCE(1264);
      END_STATE();
    case 1258:
      if (lookahead == 'k') ADVANCE(1265);
      END_STATE();
    case 1259:
      if (lookahead == 'r') ADVANCE(1266);
      END_STATE();
    case 1260:
      if (lookahead == 't') ADVANCE(1267);
      END_STATE();
    case 1261:
      if (lookahead == 'r') ADVANCE(1268);
      END_STATE();
    case 1262:
      if (lookahead == 'i') ADVANCE(1269);
      END_STATE();
    case 1263:
      if (lookahead == 'n') ADVANCE(1270);
      END_STATE();
    case 1264:
      ACCEPT_TOKEN(anon_sym_safe_DASHarea_DASHcontainer);
      END_STATE();
    case 1265:
      ACCEPT_TOKEN(anon_sym_state_machine_block);
      END_STATE();
    case 1266:
      if (lookahead == 't') ADVANCE(1271);
      END_STATE();
    case 1267:
      ACCEPT_TOKEN(anon_sym_capture_state_report);
      END_STATE();
    case 1268:
      if (lookahead == 't') ADVANCE(1272);
      END_STATE();
    case 1269:
      if (lookahead == 'n') ADVANCE(1273);
      END_STATE();
    case 1270:
      if (lookahead == 'g') ADVANCE(1274);
      END_STATE();
    case 1271:
      ACCEPT_TOKEN(anon_sym_capture_signal_report);
      END_STATE();
    case 1272:
      ACCEPT_TOKEN(anon_sym_capture_timing_report);
      END_STATE();
    case 1273:
      if (lookahead == 'g') ADVANCE(1275);
      END_STATE();
    case 1274:
      ACCEPT_TOKEN(anon_sym_enable_state_tracking);
      END_STATE();
    case 1275:
      ACCEPT_TOKEN(anon_sym_enable_signal_tracking);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0, .external_lex_state = 1},
  [1] = {.lex_state = 44},
  [2] = {.lex_state = 42},
  [3] = {.lex_state = 42},
  [4] = {.lex_state = 42},
  [5] = {.lex_state = 1},
  [6] = {.lex_state = 1},
  [7] = {.lex_state = 1},
  [8] = {.lex_state = 44},
  [9] = {.lex_state = 1},
  [10] = {.lex_state = 1},
  [11] = {.lex_state = 3},
  [12] = {.lex_state = 1},
  [13] = {.lex_state = 1},
  [14] = {.lex_state = 1},
  [15] = {.lex_state = 1},
  [16] = {.lex_state = 1},
  [17] = {.lex_state = 4},
  [18] = {.lex_state = 1},
  [19] = {.lex_state = 4},
  [20] = {.lex_state = 1},
  [21] = {.lex_state = 44},
  [22] = {.lex_state = 3},
  [23] = {.lex_state = 1},
  [24] = {.lex_state = 1},
  [25] = {.lex_state = 1},
  [26] = {.lex_state = 6},
  [27] = {.lex_state = 6},
  [28] = {.lex_state = 45},
  [29] = {.lex_state = 43},
  [30] = {.lex_state = 6},
  [31] = {.lex_state = 6},
  [32] = {.lex_state = 43},
  [33] = {.lex_state = 43},
  [34] = {.lex_state = 45},
  [35] = {.lex_state = 43},
  [36] = {.lex_state = 43},
  [37] = {.lex_state = 43},
  [38] = {.lex_state = 6},
  [39] = {.lex_state = 43},
  [40] = {.lex_state = 43},
  [41] = {.lex_state = 43},
  [42] = {.lex_state = 43},
  [43] = {.lex_state = 43},
  [44] = {.lex_state = 43},
  [45] = {.lex_state = 43},
  [46] = {.lex_state = 43},
  [47] = {.lex_state = 43},
  [48] = {.lex_state = 6},
  [49] = {.lex_state = 6},
  [50] = {.lex_state = 45},
  [51] = {.lex_state = 45},
  [52] = {.lex_state = 5},
  [53] = {.lex_state = 3},
  [54] = {.lex_state = 3},
  [55] = {.lex_state = 5},
  [56] = {.lex_state = 4},
  [57] = {.lex_state = 5},
  [58] = {.lex_state = 5},
  [59] = {.lex_state = 5},
  [60] = {.lex_state = 5},
  [61] = {.lex_state = 42},
  [62] = {.lex_state = 5},
  [63] = {.lex_state = 5},
  [64] = {.lex_state = 4},
  [65] = {.lex_state = 5},
  [66] = {.lex_state = 5},
  [67] = {.lex_state = 5},
  [68] = {.lex_state = 5},
  [69] = {.lex_state = 5},
  [70] = {.lex_state = 5},
  [71] = {.lex_state = 5},
  [72] = {.lex_state = 5},
  [73] = {.lex_state = 5},
  [74] = {.lex_state = 5},
  [75] = {.lex_state = 5},
  [76] = {.lex_state = 2},
  [77] = {.lex_state = 5},
  [78] = {.lex_state = 5},
  [79] = {.lex_state = 5},
  [80] = {.lex_state = 6},
  [81] = {.lex_state = 5},
  [82] = {.lex_state = 6},
  [83] = {.lex_state = 6},
  [84] = {.lex_state = 6},
  [85] = {.lex_state = 5},
  [86] = {.lex_state = 2},
  [87] = {.lex_state = 5},
  [88] = {.lex_state = 5},
  [89] = {.lex_state = 5},
  [90] = {.lex_state = 5},
  [91] = {.lex_state = 5},
  [92] = {.lex_state = 5},
  [93] = {.lex_state = 6},
  [94] = {.lex_state = 5},
  [95] = {.lex_state = 5},
  [96] = {.lex_state = 6},
  [97] = {.lex_state = 45},
  [98] = {.lex_state = 45},
  [99] = {.lex_state = 6},
  [100] = {.lex_state = 2},
  [101] = {.lex_state = 5},
  [102] = {.lex_state = 2},
  [103] = {.lex_state = 5},
  [104] = {.lex_state = 2},
  [105] = {.lex_state = 2},
  [106] = {.lex_state = 2},
  [107] = {.lex_state = 45},
  [108] = {.lex_state = 45},
  [109] = {.lex_state = 45},
  [110] = {.lex_state = 45},
  [111] = {.lex_state = 2},
  [112] = {.lex_state = 2},
  [113] = {.lex_state = 45},
  [114] = {.lex_state = 2},
  [115] = {.lex_state = 2},
  [116] = {.lex_state = 2},
  [117] = {.lex_state = 2},
  [118] = {.lex_state = 2},
  [119] = {.lex_state = 2},
  [120] = {.lex_state = 4},
  [121] = {.lex_state = 3},
  [122] = {.lex_state = 4},
  [123] = {.lex_state = 9},
  [124] = {.lex_state = 9},
  [125] = {.lex_state = 4},
  [126] = {.lex_state = 3},
  [127] = {.lex_state = 3},
  [128] = {.lex_state = 9},
  [129] = {.lex_state = 3},
  [130] = {.lex_state = 3},
  [131] = {.lex_state = 3},
  [132] = {.lex_state = 3},
  [133] = {.lex_state = 4},
  [134] = {.lex_state = 3},
  [135] = {.lex_state = 44},
  [136] = {.lex_state = 4},
  [137] = {.lex_state = 9},
  [138] = {.lex_state = 44},
  [139] = {.lex_state = 4},
  [140] = {.lex_state = 4},
  [141] = {.lex_state = 9},
  [142] = {.lex_state = 4},
  [143] = {.lex_state = 3},
  [144] = {.lex_state = 5},
  [145] = {.lex_state = 5},
  [146] = {.lex_state = 4},
  [147] = {.lex_state = 5},
  [148] = {.lex_state = 3},
  [149] = {.lex_state = 4},
  [150] = {.lex_state = 42},
  [151] = {.lex_state = 42},
  [152] = {.lex_state = 42},
  [153] = {.lex_state = 8},
  [154] = {.lex_state = 10},
  [155] = {.lex_state = 10},
  [156] = {.lex_state = 42},
  [157] = {.lex_state = 42},
  [158] = {.lex_state = 10},
  [159] = {.lex_state = 10},
  [160] = {.lex_state = 8},
  [161] = {.lex_state = 42},
  [162] = {.lex_state = 42},
  [163] = {.lex_state = 9},
  [164] = {.lex_state = 8},
  [165] = {.lex_state = 9},
  [166] = {.lex_state = 9},
  [167] = {.lex_state = 8},
  [168] = {.lex_state = 9},
  [169] = {.lex_state = 9},
  [170] = {.lex_state = 8},
  [171] = {.lex_state = 0},
  [172] = {.lex_state = 0},
  [173] = {.lex_state = 45},
  [174] = {.lex_state = 0},
  [175] = {.lex_state = 42},
  [176] = {.lex_state = 42},
  [177] = {.lex_state = 10},
  [178] = {.lex_state = 10},
  [179] = {.lex_state = 8},
  [180] = {.lex_state = 0},
  [181] = {.lex_state = 8},
  [182] = {.lex_state = 9},
  [183] = {.lex_state = 9},
  [184] = {.lex_state = 9},
  [185] = {.lex_state = 9},
  [186] = {.lex_state = 9},
  [187] = {.lex_state = 9},
  [188] = {.lex_state = 9},
  [189] = {.lex_state = 10},
  [190] = {.lex_state = 42},
  [191] = {.lex_state = 10},
  [192] = {.lex_state = 9},
  [193] = {.lex_state = 10},
  [194] = {.lex_state = 9},
  [195] = {.lex_state = 44},
  [196] = {.lex_state = 9},
  [197] = {.lex_state = 44},
  [198] = {.lex_state = 9},
  [199] = {.lex_state = 9},
  [200] = {.lex_state = 9},
  [201] = {.lex_state = 44},
  [202] = {.lex_state = 44},
  [203] = {.lex_state = 9},
  [204] = {.lex_state = 42},
  [205] = {.lex_state = 44},
  [206] = {.lex_state = 9},
  [207] = {.lex_state = 44},
  [208] = {.lex_state = 44},
  [209] = {.lex_state = 44},
  [210] = {.lex_state = 44},
  [211] = {.lex_state = 9},
  [212] = {.lex_state = 9},
  [213] = {.lex_state = 44},
  [214] = {.lex_state = 9},
  [215] = {.lex_state = 9},
  [216] = {.lex_state = 9},
  [217] = {.lex_state = 44},
  [218] = {.lex_state = 9},
  [219] = {.lex_state = 44},
  [220] = {.lex_state = 9},
  [221] = {.lex_state = 9},
  [222] = {.lex_state = 44},
  [223] = {.lex_state = 9},
  [224] = {.lex_state = 44},
  [225] = {.lex_state = 44},
  [226] = {.lex_state = 9},
  [227] = {.lex_state = 9},
  [228] = {.lex_state = 9},
  [229] = {.lex_state = 44},
  [230] = {.lex_state = 44},
  [231] = {.lex_state = 44},
  [232] = {.lex_state = 9},
  [233] = {.lex_state = 44},
  [234] = {.lex_state = 9},
  [235] = {.lex_state = 44},
  [236] = {.lex_state = 44},
  [237] = {.lex_state = 44},
  [238] = {.lex_state = 9},
  [239] = {.lex_state = 44},
  [240] = {.lex_state = 44},
  [241] = {.lex_state = 44},
  [242] = {.lex_state = 2},
  [243] = {.lex_state = 0},
  [244] = {.lex_state = 44},
  [245] = {.lex_state = 44},
  [246] = {.lex_state = 44},
  [247] = {.lex_state = 44},
  [248] = {.lex_state = 44},
  [249] = {.lex_state = 44},
  [250] = {.lex_state = 44},
  [251] = {.lex_state = 9},
  [252] = {.lex_state = 44},
  [253] = {.lex_state = 44},
  [254] = {.lex_state = 44},
  [255] = {.lex_state = 44},
  [256] = {.lex_state = 44},
  [257] = {.lex_state = 44},
  [258] = {.lex_state = 44},
  [259] = {.lex_state = 44},
  [260] = {.lex_state = 44},
  [261] = {.lex_state = 2},
  [262] = {.lex_state = 44},
  [263] = {.lex_state = 2},
  [264] = {.lex_state = 13},
  [265] = {.lex_state = 44},
  [266] = {.lex_state = 9},
  [267] = {.lex_state = 9},
  [268] = {.lex_state = 0},
  [269] = {.lex_state = 2},
  [270] = {.lex_state = 9},
  [271] = {.lex_state = 9},
  [272] = {.lex_state = 9},
  [273] = {.lex_state = 9},
  [274] = {.lex_state = 9},
  [275] = {.lex_state = 0},
  [276] = {.lex_state = 9},
  [277] = {.lex_state = 9},
  [278] = {.lex_state = 9},
  [279] = {.lex_state = 9},
  [280] = {.lex_state = 9},
  [281] = {.lex_state = 0},
  [282] = {.lex_state = 0},
  [283] = {.lex_state = 0},
  [284] = {.lex_state = 0, .external_lex_state = 1},
  [285] = {.lex_state = 0},
  [286] = {.lex_state = 0},
  [287] = {.lex_state = 0},
  [288] = {.lex_state = 0},
  [289] = {.lex_state = 44},
  [290] = {.lex_state = 44},
  [291] = {.lex_state = 44},
  [292] = {.lex_state = 0},
  [293] = {.lex_state = 44},
  [294] = {.lex_state = 44},
  [295] = {.lex_state = 44},
  [296] = {.lex_state = 44},
  [297] = {.lex_state = 44},
  [298] = {.lex_state = 44},
  [299] = {.lex_state = 0},
  [300] = {.lex_state = 0},
  [301] = {.lex_state = 15},
  [302] = {.lex_state = 0},
  [303] = {.lex_state = 0},
  [304] = {.lex_state = 44},
  [305] = {.lex_state = 44},
  [306] = {.lex_state = 0},
  [307] = {.lex_state = 0},
  [308] = {.lex_state = 44},
  [309] = {.lex_state = 44},
  [310] = {.lex_state = 44},
  [311] = {.lex_state = 44},
  [312] = {.lex_state = 0},
  [313] = {.lex_state = 44},
  [314] = {.lex_state = 44},
  [315] = {.lex_state = 44},
  [316] = {.lex_state = 0},
  [317] = {.lex_state = 44},
  [318] = {.lex_state = 0},
  [319] = {.lex_state = 44},
  [320] = {.lex_state = 44},
  [321] = {.lex_state = 44},
  [322] = {.lex_state = 0},
  [323] = {.lex_state = 0},
  [324] = {.lex_state = 44},
  [325] = {.lex_state = 44},
  [326] = {.lex_state = 44},
  [327] = {.lex_state = 0},
  [328] = {.lex_state = 0},
  [329] = {.lex_state = 44},
  [330] = {.lex_state = 44},
  [331] = {.lex_state = 44},
  [332] = {.lex_state = 44},
  [333] = {.lex_state = 44},
  [334] = {.lex_state = 44},
  [335] = {.lex_state = 44},
  [336] = {.lex_state = 44},
  [337] = {.lex_state = 0},
  [338] = {.lex_state = 0},
  [339] = {.lex_state = 100},
  [340] = {.lex_state = 0},
  [341] = {.lex_state = 44},
  [342] = {.lex_state = 0},
  [343] = {(TSStateId)(-1)},
  [344] = {(TSStateId)(-1)},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [sym_line_comment] = STATE(0),
    [sym_block_comment] = STATE(0),
    [ts_builtin_sym_end] = ACTIONS(1),
    [sym_identifier] = ACTIONS(1),
    [anon_sym_toast] = ACTIONS(1),
    [anon_sym_wvisible] = ACTIONS(1),
    [anon_sym_when] = ACTIONS(1),
    [anon_sym_capture] = ACTIONS(1),
    [anon_sym_wait_until] = ACTIONS(1),
    [anon_sym_then] = ACTIONS(1),
    [anon_sym_mouse] = ACTIONS(1),
    [anon_sym_drawer] = ACTIONS(1),
    [anon_sym_find] = ACTIONS(1),
    [anon_sym_cart_DASHitems] = ACTIONS(1),
    [anon_sym_mutate] = ACTIONS(1),
    [anon_sym_mobile_DASHheading] = ACTIONS(1),
    [anon_sym_container] = ACTIONS(1),
    [anon_sym_view] = ACTIONS(1),
    [anon_sym_presence] = ACTIONS(1),
    [anon_sym_wait_DASHfor] = ACTIONS(1),
    [anon_sym_data] = ACTIONS(1),
    [anon_sym_example] = ACTIONS(1),
    [anon_sym_transition] = ACTIONS(1),
    [anon_sym_mount] = ACTIONS(1),
    [anon_sym_font] = ACTIONS(1),
    [anon_sym_react_DASHto_DASHintensity] = ACTIONS(1),
    [anon_sym_state] = ACTIONS(1),
    [anon_sym_clock] = ACTIONS(1),
    [anon_sym_landscape] = ACTIONS(1),
    [anon_sym_on_DASHvisible] = ACTIONS(1),
    [anon_sym_camera_DASHscan] = ACTIONS(1),
    [anon_sym_location] = ACTIONS(1),
    [anon_sym_time] = ACTIONS(1),
    [anon_sym_apply] = ACTIONS(1),
    [anon_sym_realtime] = ACTIONS(1),
    [anon_sym_clear_DASHmocks] = ACTIONS(1),
    [anon_sym_mock_DASHresponse] = ACTIONS(1),
    [anon_sym_enable_signal_tracking] = ACTIONS(1),
    [anon_sym_fade_DASHin_DASHup] = ACTIONS(1),
    [anon_sym_socket] = ACTIONS(1),
    [anon_sym_input] = ACTIONS(1),
    [anon_sym_morph_DASHto_DASHclick] = ACTIONS(1),
    [anon_sym_replace] = ACTIONS(1),
    [anon_sym_pull_DASHrefresh] = ACTIONS(1),
    [anon_sym_distort] = ACTIONS(1),
    [anon_sym_shader] = ACTIONS(1),
    [anon_sym_given] = ACTIONS(1),
    [anon_sym_pan] = ACTIONS(1),
    [anon_sym_assert] = ACTIONS(1),
    [anon_sym_mobile_DASHtokens] = ACTIONS(1),
    [anon_sym_touch_DASHlong_DASHpress] = ACTIONS(1),
    [anon_sym_preset] = ACTIONS(1),
    [anon_sym_swipe] = ACTIONS(1),
    [anon_sym_editable_DASHmark] = ACTIONS(1),
    [anon_sym_wload] = ACTIONS(1),
    [anon_sym_observe] = ACTIONS(1),
    [anon_sym_long_DASHpress] = ACTIONS(1),
    [anon_sym_form] = ACTIONS(1),
    [anon_sym_cursor] = ACTIONS(1),
    [anon_sym_smooth_DASHscroll] = ACTIONS(1),
    [anon_sym_whover] = ACTIONS(1),
    [anon_sym_on] = ACTIONS(1),
    [anon_sym_mobile_DASHswipe] = ACTIONS(1),
    [anon_sym_mobile_DASHtext] = ACTIONS(1),
    [anon_sym_reveal] = ACTIONS(1),
    [anon_sym_on_DASHmutation] = ACTIONS(1),
    [anon_sym_ios] = ACTIONS(1),
    [anon_sym_capture_state_report] = ACTIONS(1),
    [anon_sym_program] = ACTIONS(1),
    [anon_sym_media] = ACTIONS(1),
    [anon_sym_snapshot] = ACTIONS(1),
    [anon_sym_test_DASHskip] = ACTIONS(1),
    [anon_sym_fade_DASHin_DASHdown] = ACTIONS(1),
    [anon_sym_eval] = ACTIONS(1),
    [anon_sym_mobile_DASHmotion] = ACTIONS(1),
    [anon_sym_wscroll] = ACTIONS(1),
    [anon_sym_else] = ACTIONS(1),
    [anon_sym_match] = ACTIONS(1),
    [anon_sym_resize] = ACTIONS(1),
    [anon_sym_each] = ACTIONS(1),
    [anon_sym_mobile_DASHinput] = ACTIONS(1),
    [anon_sym_test_list] = ACTIONS(1),
    [anon_sym_after] = ACTIONS(1),
    [anon_sym_morph_DASHto_DASHradius] = ACTIONS(1),
    [anon_sym_magnetic_DASHglow] = ACTIONS(1),
    [anon_sym_fill] = ACTIONS(1),
    [anon_sym_import] = ACTIONS(1),
    [anon_sym_particle_DASHfield] = ACTIONS(1),
    [anon_sym_record_DASHtimeline] = ACTIONS(1),
    [anon_sym_assert_DASHmock_DASHcalled] = ACTIONS(1),
    [anon_sym_effect] = ACTIONS(1),
    [anon_sym_video_DASHvisibility] = ACTIONS(1),
    [anon_sym_wloop] = ACTIONS(1),
    [anon_sym_biometric] = ACTIONS(1),
    [anon_sym_touch_DASHpinch] = ACTIONS(1),
    [anon_sym_notification] = ACTIONS(1),
    [anon_sym_sequence] = ACTIONS(1),
    [anon_sym_safe_DASHarea_DASHcontainer] = ACTIONS(1),
    [anon_sym_mobile_DASHtextarea] = ACTIONS(1),
    [anon_sym_breakpoint] = ACTIONS(1),
    [anon_sym_pinch] = ACTIONS(1),
    [anon_sym_touch_DASHstate] = ACTIONS(1),
    [anon_sym_run] = ACTIONS(1),
    [anon_sym_sheet] = ACTIONS(1),
    [anon_sym_richtext] = ACTIONS(1),
    [anon_sym_morph_DASHto_DASHintensity] = ACTIONS(1),
    [anon_sym_fn] = ACTIONS(1),
    [anon_sym_on_DASHhover] = ACTIONS(1),
    [anon_sym_scroll] = ACTIONS(1),
    [anon_sym_replace_DASHregex] = ACTIONS(1),
    [anon_sym_enable_timing] = ACTIONS(1),
    [anon_sym_in] = ACTIONS(1),
    [anon_sym_modal] = ACTIONS(1),
    [anon_sym_type] = ACTIONS(1),
    [anon_sym_pulse_DASHfalloff] = ACTIONS(1),
    [anon_sym_tabs] = ACTIONS(1),
    [anon_sym_mobile] = ACTIONS(1),
    [anon_sym_drag] = ACTIONS(1),
    [anon_sym_morph_DASHto_DASHcolor] = ACTIONS(1),
    [anon_sym_wait_for_signal] = ACTIONS(1),
    [anon_sym_test] = ACTIONS(1),
    [anon_sym_mock] = ACTIONS(1),
    [anon_sym_on_DASHclick] = ACTIONS(1),
    [anon_sym_cleanup] = ACTIONS(1),
    [anon_sym_timeline] = ACTIONS(1),
    [anon_sym_network_DASHset] = ACTIONS(1),
    [anon_sym_touch_DASHswipe] = ACTIONS(1),
    [anon_sym_include] = ACTIONS(1),
    [anon_sym_mobile_DASHtypography] = ACTIONS(1),
    [anon_sym_model] = ACTIONS(1),
    [anon_sym_react_DASHto_DASHcenter] = ACTIONS(1),
    [anon_sym_wait_for_state] = ACTIONS(1),
    [anon_sym_capture_signal_report] = ACTIONS(1),
    [anon_sym_camera] = ACTIONS(1),
    [anon_sym_device_DASHframe] = ACTIONS(1),
    [anon_sym_mock_DASHnative] = ACTIONS(1),
    [anon_sym_touch_DASHdrag] = ACTIONS(1),
    [anon_sym_mock_DASHpermission] = ACTIONS(1),
    [anon_sym_async_transition] = ACTIONS(1),
    [anon_sym_device] = ACTIONS(1),
    [anon_sym_mobile_DASHbutton] = ACTIONS(1),
    [anon_sym_portal] = ACTIONS(1),
    [anon_sym_mobile_DASHcolors_DASHdark] = ACTIONS(1),
    [anon_sym_click] = ACTIONS(1),
    [anon_sym_touch_DASHtap] = ACTIONS(1),
    [anon_sym_wait] = ACTIONS(1),
    [anon_sym_on_DASHfocus] = ACTIONS(1),
    [anon_sym_fuzz] = ACTIONS(1),
    [anon_sym_output_DASHjson] = ACTIONS(1),
    [anon_sym_offline] = ACTIONS(1),
    [anon_sym_fixture] = ACTIONS(1),
    [anon_sym_on_DASHintersect] = ACTIONS(1),
    [anon_sym_print] = ACTIONS(1),
    [anon_sym_out] = ACTIONS(1),
    [anon_sym_enable_state_tracking] = ACTIONS(1),
    [anon_sym_scroll_DASHview] = ACTIONS(1),
    [anon_sym_native] = ACTIONS(1),
    [anon_sym_scene] = ACTIONS(1),
    [anon_sym_state_machine_block] = ACTIONS(1),
    [anon_sym_hover] = ACTIONS(1),
    [anon_sym_haptic] = ACTIONS(1),
    [anon_sym_editable] = ACTIONS(1),
    [anon_sym_test_DASHonly] = ACTIONS(1),
    [anon_sym_device_DASHcustom] = ACTIONS(1),
    [anon_sym_mobile_DASHtheme] = ACTIONS(1),
    [anon_sym_ws_DASHon] = ACTIONS(1),
    [anon_sym_cart_DASHcount] = ACTIONS(1),
    [anon_sym_cycle] = ACTIONS(1),
    [anon_sym_for] = ACTIONS(1),
    [anon_sym_light] = ACTIONS(1),
    [anon_sym_fade_DASHin_DASHstagger] = ACTIONS(1),
    [anon_sym_morph_DASHto] = ACTIONS(1),
    [anon_sym_channel] = ACTIONS(1),
    [anon_sym_mobile_DASHsearch] = ACTIONS(1),
    [anon_sym_slow_DASHnetwork] = ACTIONS(1),
    [anon_sym_mobile_DASHinput_DASHfield] = ACTIONS(1),
    [anon_sym_if] = ACTIONS(1),
    [anon_sym_use] = ACTIONS(1),
    [anon_sym_mobile_DASHcolors] = ACTIONS(1),
    [anon_sym_script] = ACTIONS(1),
    [anon_sym_mouse_DASHposition] = ACTIONS(1),
    [anon_sym_show] = ACTIONS(1),
    [anon_sym_surface] = ACTIONS(1),
    [anon_sym_morph_DASHto_DASHfalloff] = ACTIONS(1),
    [anon_sym_property] = ACTIONS(1),
    [anon_sym_magnetic] = ACTIONS(1),
    [anon_sym_react_DASHto_DASHradius] = ACTIONS(1),
    [anon_sym_safe_DASHarea_DASHinset] = ACTIONS(1),
    [anon_sym_repeat] = ACTIONS(1),
    [anon_sym_chain] = ACTIONS(1),
    [anon_sym_locale] = ACTIONS(1),
    [anon_sym_dark] = ACTIONS(1),
    [anon_sym_compute] = ACTIONS(1),
    [anon_sym_state_machine] = ACTIONS(1),
    [anon_sym_capture_timing_report] = ACTIONS(1),
    [anon_sym_load] = ACTIONS(1),
    [anon_sym_morph_DASHto_DASHglow] = ACTIONS(1),
    [anon_sym_behavior] = ACTIONS(1),
    [anon_sym_share] = ACTIONS(1),
    [anon_sym_cart_DASHtotal] = ACTIONS(1),
    [anon_sym_value_DASHchange] = ACTIONS(1),
    [anon_sym_fade_DASHin] = ACTIONS(1),
    [anon_sym_output] = ACTIONS(1),
    [anon_sym_android] = ACTIONS(1),
    [anon_sym_loop] = ACTIONS(1),
    [anon_sym_portrait] = ACTIONS(1),
    [anon_sym_pulse_DASHradius] = ACTIONS(1),
    [anon_sym_bind] = ACTIONS(1),
    [anon_sym_try] = ACTIONS(1),
    [anon_sym_drive] = ACTIONS(1),
    [anon_sym_log] = ACTIONS(1),
    [anon_sym_scroll_DASHspy] = ACTIONS(1),
    [anon_sym_websocket] = ACTIONS(1),
    [anon_sym_let] = ACTIONS(1),
    [anon_sym_network] = ACTIONS(1),
    [anon_sym_editable_DASHblock] = ACTIONS(1),
    [anon_sym_touch_DASHpreset] = ACTIONS(1),
    [anon_sym_template] = ACTIONS(1),
    [anon_sym_swarm] = ACTIONS(1),
    [anon_sym_persist] = ACTIONS(1),
    [anon_sym_won] = ACTIONS(1),
    [anon_sym_stack] = ACTIONS(1),
    [anon_sym_pulse_DASHintensity] = ACTIONS(1),
    [anon_sym_wclick] = ACTIONS(1),
    [anon_sym_flush] = ACTIONS(1),
    [anon_sym_wait_for_timeline] = ACTIONS(1),
    [anon_sym_safe_DASHarea] = ACTIONS(1),
    [anon_sym_connect] = ACTIONS(1),
    [anon_sym_error_list] = ACTIONS(1),
    [anon_sym_reduced_DASHmotion] = ACTIONS(1),
    [anon_sym_AT] = ACTIONS(1),
    [anon_sym_SEMI] = ACTIONS(1),
    [anon_sym_LPAREN] = ACTIONS(1),
    [anon_sym_RPAREN] = ACTIONS(1),
    [anon_sym_COLON] = ACTIONS(1),
    [anon_sym_COMMA] = ACTIONS(1),
    [anon_sym_as] = ACTIONS(1),
    [anon_sym_EQ] = ACTIONS(1),
    [anon_sym_PERCENT] = ACTIONS(1),
    [anon_sym_emit] = ACTIONS(1),
    [anon_sym_LBRACE] = ACTIONS(1),
    [anon_sym_RBRACE] = ACTIONS(1),
    [anon_sym_macro] = ACTIONS(1),
    [anon_sym_primitive] = ACTIONS(1),
    [anon_sym_capture_type] = ACTIONS(1),
    [anon_sym_captureType] = ACTIONS(1),
    [anon_sym_DASH_GT] = ACTIONS(1),
    [anon_sym_PLUS] = ACTIONS(1),
    [anon_sym_DASH] = ACTIONS(1),
    [anon_sym_STAR] = ACTIONS(1),
    [anon_sym_SLASH] = ACTIONS(1),
    [anon_sym_EQ_EQ_EQ] = ACTIONS(1),
    [anon_sym_BANG_EQ_EQ] = ACTIONS(1),
    [anon_sym_EQ_EQ] = ACTIONS(1),
    [anon_sym_BANG_EQ] = ACTIONS(1),
    [anon_sym_LT] = ACTIONS(1),
    [anon_sym_GT] = ACTIONS(1),
    [anon_sym_LT_EQ] = ACTIONS(1),
    [anon_sym_GT_EQ] = ACTIONS(1),
    [anon_sym_AMP_AMP] = ACTIONS(1),
    [anon_sym_PIPE_PIPE] = ACTIONS(1),
    [anon_sym_AMP] = ACTIONS(1),
    [anon_sym_is] = ACTIONS(1),
    [anon_sym_cubic_DASHbezier] = ACTIONS(1),
    [anon_sym_QMARK] = ACTIONS(1),
    [anon_sym_from] = ACTIONS(1),
    [anon_sym_to] = ACTIONS(1),
    [anon_sym_PLUS_EQ] = ACTIONS(1),
    [anon_sym_DASH_EQ] = ACTIONS(1),
    [anon_sym_STAR_EQ] = ACTIONS(1),
    [anon_sym_SLASH_EQ] = ACTIONS(1),
    [anon_sym_SLASH_GT] = ACTIONS(1),
    [anon_sym_LT_SLASH] = ACTIONS(1),
    [anon_sym_SLASH_SLASH] = ACTIONS(3),
    [anon_sym_SLASH_STAR] = ACTIONS(5),
    [anon_sym_DOLLAR] = ACTIONS(1),
    [anon_sym_TILDE] = ACTIONS(1),
    [sym_selector] = ACTIONS(1),
    [sym_string] = ACTIONS(1),
    [sym_template_string] = ACTIONS(1),
    [sym_percentage] = ACTIONS(1),
    [sym_color] = ACTIONS(1),
    [sym_emit_content] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(340),
    [sym__item] = STATE(258),
    [sym_directive] = STATE(256),
    [sym_generic_directive] = STATE(256),
    [sym_emit_directive] = STATE(256),
    [sym_macro_def] = STATE(256),
    [sym_primitive_def] = STATE(256),
    [sym_capture_type_def] = STATE(256),
    [sym_meta_directive] = STATE(256),
    [sym_scope_block] = STATE(256),
    [sym_selector_list] = STATE(287),
    [sym_line_comment] = STATE(1),
    [sym_block_comment] = STATE(1),
    [aux_sym_source_file_repeat1] = STATE(138),
    [aux_sym_selector_list_repeat1] = STATE(243),
    [ts_builtin_sym_end] = ACTIONS(7),
    [sym_identifier] = ACTIONS(9),
    [anon_sym_AT] = ACTIONS(11),
    [anon_sym_PERCENT] = ACTIONS(13),
    [anon_sym_SLASH_SLASH] = ACTIONS(3),
    [anon_sym_SLASH_STAR] = ACTIONS(5),
    [sym_selector] = ACTIONS(15),
  },
  [2] = {
    [sym__known_directive_name] = STATE(27),
    [sym_line_comment] = STATE(2),
    [sym_block_comment] = STATE(2),
    [sym_identifier] = ACTIONS(17),
    [anon_sym_toast] = ACTIONS(19),
    [anon_sym_wvisible] = ACTIONS(19),
    [anon_sym_when] = ACTIONS(19),
    [anon_sym_capture] = ACTIONS(19),
    [anon_sym_wait_until] = ACTIONS(19),
    [anon_sym_then] = ACTIONS(19),
    [anon_sym_mouse] = ACTIONS(19),
    [anon_sym_drawer] = ACTIONS(19),
    [anon_sym_find] = ACTIONS(19),
    [anon_sym_cart_DASHitems] = ACTIONS(19),
    [anon_sym_mutate] = ACTIONS(19),
    [anon_sym_mobile_DASHheading] = ACTIONS(19),
    [anon_sym_container] = ACTIONS(19),
    [anon_sym_view] = ACTIONS(19),
    [anon_sym_presence] = ACTIONS(19),
    [anon_sym_wait_DASHfor] = ACTIONS(19),
    [anon_sym_data] = ACTIONS(19),
    [anon_sym_example] = ACTIONS(19),
    [anon_sym_transition] = ACTIONS(19),
    [anon_sym_mount] = ACTIONS(19),
    [anon_sym_font] = ACTIONS(19),
    [anon_sym_react_DASHto_DASHintensity] = ACTIONS(19),
    [anon_sym_state] = ACTIONS(19),
    [anon_sym_clock] = ACTIONS(19),
    [anon_sym_landscape] = ACTIONS(19),
    [anon_sym_on_DASHvisible] = ACTIONS(19),
    [anon_sym_camera_DASHscan] = ACTIONS(19),
    [anon_sym_location] = ACTIONS(19),
    [anon_sym_time] = ACTIONS(19),
    [anon_sym_apply] = ACTIONS(19),
    [anon_sym_realtime] = ACTIONS(19),
    [anon_sym_clear_DASHmocks] = ACTIONS(19),
    [anon_sym_mock_DASHresponse] = ACTIONS(19),
    [anon_sym_enable_signal_tracking] = ACTIONS(19),
    [anon_sym_fade_DASHin_DASHup] = ACTIONS(19),
    [anon_sym_socket] = ACTIONS(19),
    [anon_sym_input] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHclick] = ACTIONS(19),
    [anon_sym_replace] = ACTIONS(19),
    [anon_sym_pull_DASHrefresh] = ACTIONS(19),
    [anon_sym_distort] = ACTIONS(19),
    [anon_sym_shader] = ACTIONS(19),
    [anon_sym_given] = ACTIONS(19),
    [anon_sym_pan] = ACTIONS(19),
    [anon_sym_assert] = ACTIONS(19),
    [anon_sym_mobile_DASHtokens] = ACTIONS(19),
    [anon_sym_touch_DASHlong_DASHpress] = ACTIONS(19),
    [anon_sym_preset] = ACTIONS(19),
    [anon_sym_swipe] = ACTIONS(19),
    [anon_sym_editable_DASHmark] = ACTIONS(19),
    [anon_sym_wload] = ACTIONS(19),
    [anon_sym_observe] = ACTIONS(19),
    [anon_sym_long_DASHpress] = ACTIONS(19),
    [anon_sym_form] = ACTIONS(19),
    [anon_sym_cursor] = ACTIONS(19),
    [anon_sym_smooth_DASHscroll] = ACTIONS(19),
    [anon_sym_whover] = ACTIONS(19),
    [anon_sym_on] = ACTIONS(19),
    [anon_sym_mobile_DASHswipe] = ACTIONS(19),
    [anon_sym_mobile_DASHtext] = ACTIONS(19),
    [anon_sym_reveal] = ACTIONS(19),
    [anon_sym_on_DASHmutation] = ACTIONS(19),
    [anon_sym_ios] = ACTIONS(19),
    [anon_sym_capture_state_report] = ACTIONS(19),
    [anon_sym_program] = ACTIONS(19),
    [anon_sym_media] = ACTIONS(19),
    [anon_sym_snapshot] = ACTIONS(19),
    [anon_sym_test_DASHskip] = ACTIONS(19),
    [anon_sym_fade_DASHin_DASHdown] = ACTIONS(19),
    [anon_sym_eval] = ACTIONS(19),
    [anon_sym_mobile_DASHmotion] = ACTIONS(19),
    [anon_sym_wscroll] = ACTIONS(19),
    [anon_sym_else] = ACTIONS(19),
    [anon_sym_match] = ACTIONS(19),
    [anon_sym_resize] = ACTIONS(19),
    [anon_sym_each] = ACTIONS(19),
    [anon_sym_mobile_DASHinput] = ACTIONS(19),
    [anon_sym_test_list] = ACTIONS(19),
    [anon_sym_after] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHradius] = ACTIONS(19),
    [anon_sym_magnetic_DASHglow] = ACTIONS(19),
    [anon_sym_fill] = ACTIONS(19),
    [anon_sym_import] = ACTIONS(19),
    [anon_sym_particle_DASHfield] = ACTIONS(19),
    [anon_sym_record_DASHtimeline] = ACTIONS(19),
    [anon_sym_assert_DASHmock_DASHcalled] = ACTIONS(19),
    [anon_sym_effect] = ACTIONS(19),
    [anon_sym_video_DASHvisibility] = ACTIONS(19),
    [anon_sym_wloop] = ACTIONS(19),
    [anon_sym_biometric] = ACTIONS(19),
    [anon_sym_touch_DASHpinch] = ACTIONS(19),
    [anon_sym_notification] = ACTIONS(19),
    [anon_sym_sequence] = ACTIONS(19),
    [anon_sym_safe_DASHarea_DASHcontainer] = ACTIONS(19),
    [anon_sym_mobile_DASHtextarea] = ACTIONS(19),
    [anon_sym_breakpoint] = ACTIONS(19),
    [anon_sym_pinch] = ACTIONS(19),
    [anon_sym_touch_DASHstate] = ACTIONS(19),
    [anon_sym_run] = ACTIONS(19),
    [anon_sym_sheet] = ACTIONS(19),
    [anon_sym_richtext] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHintensity] = ACTIONS(19),
    [anon_sym_fn] = ACTIONS(19),
    [anon_sym_on_DASHhover] = ACTIONS(19),
    [anon_sym_scroll] = ACTIONS(19),
    [anon_sym_replace_DASHregex] = ACTIONS(19),
    [anon_sym_enable_timing] = ACTIONS(19),
    [anon_sym_in] = ACTIONS(19),
    [anon_sym_modal] = ACTIONS(19),
    [anon_sym_type] = ACTIONS(19),
    [anon_sym_pulse_DASHfalloff] = ACTIONS(19),
    [anon_sym_tabs] = ACTIONS(19),
    [anon_sym_mobile] = ACTIONS(19),
    [anon_sym_drag] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHcolor] = ACTIONS(19),
    [anon_sym_wait_for_signal] = ACTIONS(19),
    [anon_sym_test] = ACTIONS(19),
    [anon_sym_mock] = ACTIONS(19),
    [anon_sym_on_DASHclick] = ACTIONS(19),
    [anon_sym_cleanup] = ACTIONS(19),
    [anon_sym_timeline] = ACTIONS(19),
    [anon_sym_network_DASHset] = ACTIONS(19),
    [anon_sym_touch_DASHswipe] = ACTIONS(19),
    [anon_sym_include] = ACTIONS(19),
    [anon_sym_mobile_DASHtypography] = ACTIONS(19),
    [anon_sym_model] = ACTIONS(19),
    [anon_sym_react_DASHto_DASHcenter] = ACTIONS(19),
    [anon_sym_wait_for_state] = ACTIONS(19),
    [anon_sym_capture_signal_report] = ACTIONS(19),
    [anon_sym_camera] = ACTIONS(19),
    [anon_sym_device_DASHframe] = ACTIONS(19),
    [anon_sym_mock_DASHnative] = ACTIONS(19),
    [anon_sym_touch_DASHdrag] = ACTIONS(19),
    [anon_sym_mock_DASHpermission] = ACTIONS(19),
    [anon_sym_async_transition] = ACTIONS(19),
    [anon_sym_device] = ACTIONS(19),
    [anon_sym_mobile_DASHbutton] = ACTIONS(19),
    [anon_sym_portal] = ACTIONS(19),
    [anon_sym_mobile_DASHcolors_DASHdark] = ACTIONS(19),
    [anon_sym_click] = ACTIONS(19),
    [anon_sym_touch_DASHtap] = ACTIONS(19),
    [anon_sym_wait] = ACTIONS(19),
    [anon_sym_on_DASHfocus] = ACTIONS(19),
    [anon_sym_fuzz] = ACTIONS(19),
    [anon_sym_output_DASHjson] = ACTIONS(19),
    [anon_sym_offline] = ACTIONS(19),
    [anon_sym_fixture] = ACTIONS(19),
    [anon_sym_on_DASHintersect] = ACTIONS(19),
    [anon_sym_print] = ACTIONS(19),
    [anon_sym_out] = ACTIONS(19),
    [anon_sym_enable_state_tracking] = ACTIONS(19),
    [anon_sym_scroll_DASHview] = ACTIONS(19),
    [anon_sym_native] = ACTIONS(19),
    [anon_sym_scene] = ACTIONS(19),
    [anon_sym_state_machine_block] = ACTIONS(19),
    [anon_sym_hover] = ACTIONS(19),
    [anon_sym_haptic] = ACTIONS(19),
    [anon_sym_editable] = ACTIONS(19),
    [anon_sym_test_DASHonly] = ACTIONS(19),
    [anon_sym_device_DASHcustom] = ACTIONS(19),
    [anon_sym_mobile_DASHtheme] = ACTIONS(19),
    [anon_sym_ws_DASHon] = ACTIONS(19),
    [anon_sym_cart_DASHcount] = ACTIONS(19),
    [anon_sym_cycle] = ACTIONS(19),
    [anon_sym_for] = ACTIONS(19),
    [anon_sym_light] = ACTIONS(19),
    [anon_sym_fade_DASHin_DASHstagger] = ACTIONS(19),
    [anon_sym_morph_DASHto] = ACTIONS(19),
    [anon_sym_channel] = ACTIONS(19),
    [anon_sym_mobile_DASHsearch] = ACTIONS(19),
    [anon_sym_slow_DASHnetwork] = ACTIONS(19),
    [anon_sym_mobile_DASHinput_DASHfield] = ACTIONS(19),
    [anon_sym_if] = ACTIONS(19),
    [anon_sym_use] = ACTIONS(19),
    [anon_sym_mobile_DASHcolors] = ACTIONS(19),
    [anon_sym_script] = ACTIONS(19),
    [anon_sym_mouse_DASHposition] = ACTIONS(19),
    [anon_sym_show] = ACTIONS(19),
    [anon_sym_surface] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHfalloff] = ACTIONS(19),
    [anon_sym_property] = ACTIONS(19),
    [anon_sym_magnetic] = ACTIONS(19),
    [anon_sym_react_DASHto_DASHradius] = ACTIONS(19),
    [anon_sym_safe_DASHarea_DASHinset] = ACTIONS(19),
    [anon_sym_repeat] = ACTIONS(19),
    [anon_sym_chain] = ACTIONS(19),
    [anon_sym_locale] = ACTIONS(19),
    [anon_sym_dark] = ACTIONS(19),
    [anon_sym_compute] = ACTIONS(19),
    [anon_sym_state_machine] = ACTIONS(19),
    [anon_sym_capture_timing_report] = ACTIONS(19),
    [anon_sym_load] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHglow] = ACTIONS(19),
    [anon_sym_behavior] = ACTIONS(19),
    [anon_sym_share] = ACTIONS(19),
    [anon_sym_cart_DASHtotal] = ACTIONS(19),
    [anon_sym_value_DASHchange] = ACTIONS(19),
    [anon_sym_fade_DASHin] = ACTIONS(19),
    [anon_sym_output] = ACTIONS(19),
    [anon_sym_android] = ACTIONS(19),
    [anon_sym_loop] = ACTIONS(19),
    [anon_sym_portrait] = ACTIONS(19),
    [anon_sym_pulse_DASHradius] = ACTIONS(19),
    [anon_sym_bind] = ACTIONS(19),
    [anon_sym_try] = ACTIONS(19),
    [anon_sym_drive] = ACTIONS(19),
    [anon_sym_log] = ACTIONS(19),
    [anon_sym_scroll_DASHspy] = ACTIONS(19),
    [anon_sym_websocket] = ACTIONS(19),
    [anon_sym_let] = ACTIONS(19),
    [anon_sym_network] = ACTIONS(19),
    [anon_sym_editable_DASHblock] = ACTIONS(19),
    [anon_sym_touch_DASHpreset] = ACTIONS(19),
    [anon_sym_template] = ACTIONS(19),
    [anon_sym_swarm] = ACTIONS(19),
    [anon_sym_persist] = ACTIONS(19),
    [anon_sym_won] = ACTIONS(19),
    [anon_sym_stack] = ACTIONS(19),
    [anon_sym_pulse_DASHintensity] = ACTIONS(19),
    [anon_sym_wclick] = ACTIONS(19),
    [anon_sym_flush] = ACTIONS(19),
    [anon_sym_wait_for_timeline] = ACTIONS(19),
    [anon_sym_safe_DASHarea] = ACTIONS(19),
    [anon_sym_connect] = ACTIONS(19),
    [anon_sym_error_list] = ACTIONS(19),
    [anon_sym_reduced_DASHmotion] = ACTIONS(19),
    [anon_sym_SLASH_SLASH] = ACTIONS(3),
    [anon_sym_SLASH_STAR] = ACTIONS(5),
  },
  [3] = {
    [sym__known_directive_name] = STATE(34),
    [sym_line_comment] = STATE(3),
    [sym_block_comment] = STATE(3),
    [sym_identifier] = ACTIONS(21),
    [anon_sym_toast] = ACTIONS(23),
    [anon_sym_wvisible] = ACTIONS(23),
    [anon_sym_when] = ACTIONS(23),
    [anon_sym_capture] = ACTIONS(23),
    [anon_sym_wait_until] = ACTIONS(23),
    [anon_sym_then] = ACTIONS(23),
    [anon_sym_mouse] = ACTIONS(23),
    [anon_sym_drawer] = ACTIONS(23),
    [anon_sym_find] = ACTIONS(23),
    [anon_sym_cart_DASHitems] = ACTIONS(23),
    [anon_sym_mutate] = ACTIONS(23),
    [anon_sym_mobile_DASHheading] = ACTIONS(23),
    [anon_sym_container] = ACTIONS(23),
    [anon_sym_view] = ACTIONS(23),
    [anon_sym_presence] = ACTIONS(23),
    [anon_sym_wait_DASHfor] = ACTIONS(23),
    [anon_sym_data] = ACTIONS(23),
    [anon_sym_example] = ACTIONS(23),
    [anon_sym_transition] = ACTIONS(23),
    [anon_sym_mount] = ACTIONS(23),
    [anon_sym_font] = ACTIONS(23),
    [anon_sym_react_DASHto_DASHintensity] = ACTIONS(23),
    [anon_sym_state] = ACTIONS(23),
    [anon_sym_clock] = ACTIONS(23),
    [anon_sym_landscape] = ACTIONS(23),
    [anon_sym_on_DASHvisible] = ACTIONS(23),
    [anon_sym_camera_DASHscan] = ACTIONS(23),
    [anon_sym_location] = ACTIONS(23),
    [anon_sym_time] = ACTIONS(23),
    [anon_sym_apply] = ACTIONS(23),
    [anon_sym_realtime] = ACTIONS(23),
    [anon_sym_clear_DASHmocks] = ACTIONS(23),
    [anon_sym_mock_DASHresponse] = ACTIONS(23),
    [anon_sym_enable_signal_tracking] = ACTIONS(23),
    [anon_sym_fade_DASHin_DASHup] = ACTIONS(23),
    [anon_sym_socket] = ACTIONS(23),
    [anon_sym_input] = ACTIONS(23),
    [anon_sym_morph_DASHto_DASHclick] = ACTIONS(23),
    [anon_sym_replace] = ACTIONS(23),
    [anon_sym_pull_DASHrefresh] = ACTIONS(23),
    [anon_sym_distort] = ACTIONS(23),
    [anon_sym_shader] = ACTIONS(23),
    [anon_sym_given] = ACTIONS(23),
    [anon_sym_pan] = ACTIONS(23),
    [anon_sym_assert] = ACTIONS(23),
    [anon_sym_mobile_DASHtokens] = ACTIONS(23),
    [anon_sym_touch_DASHlong_DASHpress] = ACTIONS(23),
    [anon_sym_preset] = ACTIONS(23),
    [anon_sym_swipe] = ACTIONS(23),
    [anon_sym_editable_DASHmark] = ACTIONS(23),
    [anon_sym_wload] = ACTIONS(23),
    [anon_sym_observe] = ACTIONS(23),
    [anon_sym_long_DASHpress] = ACTIONS(23),
    [anon_sym_form] = ACTIONS(23),
    [anon_sym_cursor] = ACTIONS(23),
    [anon_sym_smooth_DASHscroll] = ACTIONS(23),
    [anon_sym_whover] = ACTIONS(23),
    [anon_sym_on] = ACTIONS(23),
    [anon_sym_mobile_DASHswipe] = ACTIONS(23),
    [anon_sym_mobile_DASHtext] = ACTIONS(23),
    [anon_sym_reveal] = ACTIONS(23),
    [anon_sym_on_DASHmutation] = ACTIONS(23),
    [anon_sym_ios] = ACTIONS(23),
    [anon_sym_capture_state_report] = ACTIONS(23),
    [anon_sym_program] = ACTIONS(23),
    [anon_sym_media] = ACTIONS(23),
    [anon_sym_snapshot] = ACTIONS(23),
    [anon_sym_test_DASHskip] = ACTIONS(23),
    [anon_sym_fade_DASHin_DASHdown] = ACTIONS(23),
    [anon_sym_eval] = ACTIONS(23),
    [anon_sym_mobile_DASHmotion] = ACTIONS(23),
    [anon_sym_wscroll] = ACTIONS(23),
    [anon_sym_else] = ACTIONS(23),
    [anon_sym_match] = ACTIONS(23),
    [anon_sym_resize] = ACTIONS(23),
    [anon_sym_each] = ACTIONS(23),
    [anon_sym_mobile_DASHinput] = ACTIONS(23),
    [anon_sym_test_list] = ACTIONS(23),
    [anon_sym_after] = ACTIONS(23),
    [anon_sym_morph_DASHto_DASHradius] = ACTIONS(23),
    [anon_sym_magnetic_DASHglow] = ACTIONS(23),
    [anon_sym_fill] = ACTIONS(23),
    [anon_sym_import] = ACTIONS(23),
    [anon_sym_particle_DASHfield] = ACTIONS(23),
    [anon_sym_record_DASHtimeline] = ACTIONS(23),
    [anon_sym_assert_DASHmock_DASHcalled] = ACTIONS(23),
    [anon_sym_effect] = ACTIONS(23),
    [anon_sym_video_DASHvisibility] = ACTIONS(23),
    [anon_sym_wloop] = ACTIONS(23),
    [anon_sym_biometric] = ACTIONS(23),
    [anon_sym_touch_DASHpinch] = ACTIONS(23),
    [anon_sym_notification] = ACTIONS(23),
    [anon_sym_sequence] = ACTIONS(23),
    [anon_sym_safe_DASHarea_DASHcontainer] = ACTIONS(23),
    [anon_sym_mobile_DASHtextarea] = ACTIONS(23),
    [anon_sym_breakpoint] = ACTIONS(23),
    [anon_sym_pinch] = ACTIONS(23),
    [anon_sym_touch_DASHstate] = ACTIONS(23),
    [anon_sym_run] = ACTIONS(23),
    [anon_sym_sheet] = ACTIONS(23),
    [anon_sym_richtext] = ACTIONS(23),
    [anon_sym_morph_DASHto_DASHintensity] = ACTIONS(23),
    [anon_sym_fn] = ACTIONS(23),
    [anon_sym_on_DASHhover] = ACTIONS(23),
    [anon_sym_scroll] = ACTIONS(23),
    [anon_sym_replace_DASHregex] = ACTIONS(23),
    [anon_sym_enable_timing] = ACTIONS(23),
    [anon_sym_in] = ACTIONS(23),
    [anon_sym_modal] = ACTIONS(23),
    [anon_sym_type] = ACTIONS(23),
    [anon_sym_pulse_DASHfalloff] = ACTIONS(23),
    [anon_sym_tabs] = ACTIONS(23),
    [anon_sym_mobile] = ACTIONS(23),
    [anon_sym_drag] = ACTIONS(23),
    [anon_sym_morph_DASHto_DASHcolor] = ACTIONS(23),
    [anon_sym_wait_for_signal] = ACTIONS(23),
    [anon_sym_test] = ACTIONS(23),
    [anon_sym_mock] = ACTIONS(23),
    [anon_sym_on_DASHclick] = ACTIONS(23),
    [anon_sym_cleanup] = ACTIONS(23),
    [anon_sym_timeline] = ACTIONS(23),
    [anon_sym_network_DASHset] = ACTIONS(23),
    [anon_sym_touch_DASHswipe] = ACTIONS(23),
    [anon_sym_include] = ACTIONS(23),
    [anon_sym_mobile_DASHtypography] = ACTIONS(23),
    [anon_sym_model] = ACTIONS(23),
    [anon_sym_react_DASHto_DASHcenter] = ACTIONS(23),
    [anon_sym_wait_for_state] = ACTIONS(23),
    [anon_sym_capture_signal_report] = ACTIONS(23),
    [anon_sym_camera] = ACTIONS(23),
    [anon_sym_device_DASHframe] = ACTIONS(23),
    [anon_sym_mock_DASHnative] = ACTIONS(23),
    [anon_sym_touch_DASHdrag] = ACTIONS(23),
    [anon_sym_mock_DASHpermission] = ACTIONS(23),
    [anon_sym_async_transition] = ACTIONS(23),
    [anon_sym_device] = ACTIONS(23),
    [anon_sym_mobile_DASHbutton] = ACTIONS(23),
    [anon_sym_portal] = ACTIONS(23),
    [anon_sym_mobile_DASHcolors_DASHdark] = ACTIONS(23),
    [anon_sym_click] = ACTIONS(23),
    [anon_sym_touch_DASHtap] = ACTIONS(23),
    [anon_sym_wait] = ACTIONS(23),
    [anon_sym_on_DASHfocus] = ACTIONS(23),
    [anon_sym_fuzz] = ACTIONS(23),
    [anon_sym_output_DASHjson] = ACTIONS(23),
    [anon_sym_offline] = ACTIONS(23),
    [anon_sym_fixture] = ACTIONS(23),
    [anon_sym_on_DASHintersect] = ACTIONS(23),
    [anon_sym_print] = ACTIONS(23),
    [anon_sym_out] = ACTIONS(23),
    [anon_sym_enable_state_tracking] = ACTIONS(23),
    [anon_sym_scroll_DASHview] = ACTIONS(23),
    [anon_sym_native] = ACTIONS(23),
    [anon_sym_scene] = ACTIONS(23),
    [anon_sym_state_machine_block] = ACTIONS(23),
    [anon_sym_hover] = ACTIONS(23),
    [anon_sym_haptic] = ACTIONS(23),
    [anon_sym_editable] = ACTIONS(23),
    [anon_sym_test_DASHonly] = ACTIONS(23),
    [anon_sym_device_DASHcustom] = ACTIONS(23),
    [anon_sym_mobile_DASHtheme] = ACTIONS(23),
    [anon_sym_ws_DASHon] = ACTIONS(23),
    [anon_sym_cart_DASHcount] = ACTIONS(23),
    [anon_sym_cycle] = ACTIONS(23),
    [anon_sym_for] = ACTIONS(23),
    [anon_sym_light] = ACTIONS(23),
    [anon_sym_fade_DASHin_DASHstagger] = ACTIONS(23),
    [anon_sym_morph_DASHto] = ACTIONS(23),
    [anon_sym_channel] = ACTIONS(23),
    [anon_sym_mobile_DASHsearch] = ACTIONS(23),
    [anon_sym_slow_DASHnetwork] = ACTIONS(23),
    [anon_sym_mobile_DASHinput_DASHfield] = ACTIONS(23),
    [anon_sym_if] = ACTIONS(23),
    [anon_sym_use] = ACTIONS(23),
    [anon_sym_mobile_DASHcolors] = ACTIONS(23),
    [anon_sym_script] = ACTIONS(23),
    [anon_sym_mouse_DASHposition] = ACTIONS(23),
    [anon_sym_show] = ACTIONS(23),
    [anon_sym_surface] = ACTIONS(23),
    [anon_sym_morph_DASHto_DASHfalloff] = ACTIONS(23),
    [anon_sym_property] = ACTIONS(23),
    [anon_sym_magnetic] = ACTIONS(23),
    [anon_sym_react_DASHto_DASHradius] = ACTIONS(23),
    [anon_sym_safe_DASHarea_DASHinset] = ACTIONS(23),
    [anon_sym_repeat] = ACTIONS(23),
    [anon_sym_chain] = ACTIONS(23),
    [anon_sym_locale] = ACTIONS(23),
    [anon_sym_dark] = ACTIONS(23),
    [anon_sym_compute] = ACTIONS(23),
    [anon_sym_state_machine] = ACTIONS(23),
    [anon_sym_capture_timing_report] = ACTIONS(23),
    [anon_sym_load] = ACTIONS(23),
    [anon_sym_morph_DASHto_DASHglow] = ACTIONS(23),
    [anon_sym_behavior] = ACTIONS(23),
    [anon_sym_share] = ACTIONS(23),
    [anon_sym_cart_DASHtotal] = ACTIONS(23),
    [anon_sym_value_DASHchange] = ACTIONS(23),
    [anon_sym_fade_DASHin] = ACTIONS(23),
    [anon_sym_output] = ACTIONS(23),
    [anon_sym_android] = ACTIONS(23),
    [anon_sym_loop] = ACTIONS(23),
    [anon_sym_portrait] = ACTIONS(23),
    [anon_sym_pulse_DASHradius] = ACTIONS(23),
    [anon_sym_bind] = ACTIONS(23),
    [anon_sym_try] = ACTIONS(23),
    [anon_sym_drive] = ACTIONS(23),
    [anon_sym_log] = ACTIONS(23),
    [anon_sym_scroll_DASHspy] = ACTIONS(23),
    [anon_sym_websocket] = ACTIONS(23),
    [anon_sym_let] = ACTIONS(23),
    [anon_sym_network] = ACTIONS(23),
    [anon_sym_editable_DASHblock] = ACTIONS(23),
    [anon_sym_touch_DASHpreset] = ACTIONS(23),
    [anon_sym_template] = ACTIONS(23),
    [anon_sym_swarm] = ACTIONS(23),
    [anon_sym_persist] = ACTIONS(23),
    [anon_sym_won] = ACTIONS(23),
    [anon_sym_stack] = ACTIONS(23),
    [anon_sym_pulse_DASHintensity] = ACTIONS(23),
    [anon_sym_wclick] = ACTIONS(23),
    [anon_sym_flush] = ACTIONS(23),
    [anon_sym_wait_for_timeline] = ACTIONS(23),
    [anon_sym_safe_DASHarea] = ACTIONS(23),
    [anon_sym_connect] = ACTIONS(23),
    [anon_sym_error_list] = ACTIONS(23),
    [anon_sym_reduced_DASHmotion] = ACTIONS(23),
    [anon_sym_SLASH_SLASH] = ACTIONS(3),
    [anon_sym_SLASH_STAR] = ACTIONS(5),
  },
  [4] = {
    [sym__known_directive_name] = STATE(26),
    [sym_line_comment] = STATE(4),
    [sym_block_comment] = STATE(4),
    [sym_identifier] = ACTIONS(25),
    [anon_sym_toast] = ACTIONS(19),
    [anon_sym_wvisible] = ACTIONS(19),
    [anon_sym_when] = ACTIONS(19),
    [anon_sym_capture] = ACTIONS(19),
    [anon_sym_wait_until] = ACTIONS(19),
    [anon_sym_then] = ACTIONS(19),
    [anon_sym_mouse] = ACTIONS(19),
    [anon_sym_drawer] = ACTIONS(19),
    [anon_sym_find] = ACTIONS(19),
    [anon_sym_cart_DASHitems] = ACTIONS(19),
    [anon_sym_mutate] = ACTIONS(19),
    [anon_sym_mobile_DASHheading] = ACTIONS(19),
    [anon_sym_container] = ACTIONS(19),
    [anon_sym_view] = ACTIONS(19),
    [anon_sym_presence] = ACTIONS(19),
    [anon_sym_wait_DASHfor] = ACTIONS(19),
    [anon_sym_data] = ACTIONS(19),
    [anon_sym_example] = ACTIONS(19),
    [anon_sym_transition] = ACTIONS(19),
    [anon_sym_mount] = ACTIONS(19),
    [anon_sym_font] = ACTIONS(19),
    [anon_sym_react_DASHto_DASHintensity] = ACTIONS(19),
    [anon_sym_state] = ACTIONS(19),
    [anon_sym_clock] = ACTIONS(19),
    [anon_sym_landscape] = ACTIONS(19),
    [anon_sym_on_DASHvisible] = ACTIONS(19),
    [anon_sym_camera_DASHscan] = ACTIONS(19),
    [anon_sym_location] = ACTIONS(19),
    [anon_sym_time] = ACTIONS(19),
    [anon_sym_apply] = ACTIONS(19),
    [anon_sym_realtime] = ACTIONS(19),
    [anon_sym_clear_DASHmocks] = ACTIONS(19),
    [anon_sym_mock_DASHresponse] = ACTIONS(19),
    [anon_sym_enable_signal_tracking] = ACTIONS(19),
    [anon_sym_fade_DASHin_DASHup] = ACTIONS(19),
    [anon_sym_socket] = ACTIONS(19),
    [anon_sym_input] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHclick] = ACTIONS(19),
    [anon_sym_replace] = ACTIONS(19),
    [anon_sym_pull_DASHrefresh] = ACTIONS(19),
    [anon_sym_distort] = ACTIONS(19),
    [anon_sym_shader] = ACTIONS(19),
    [anon_sym_given] = ACTIONS(19),
    [anon_sym_pan] = ACTIONS(19),
    [anon_sym_assert] = ACTIONS(19),
    [anon_sym_mobile_DASHtokens] = ACTIONS(19),
    [anon_sym_touch_DASHlong_DASHpress] = ACTIONS(19),
    [anon_sym_preset] = ACTIONS(19),
    [anon_sym_swipe] = ACTIONS(19),
    [anon_sym_editable_DASHmark] = ACTIONS(19),
    [anon_sym_wload] = ACTIONS(19),
    [anon_sym_observe] = ACTIONS(19),
    [anon_sym_long_DASHpress] = ACTIONS(19),
    [anon_sym_form] = ACTIONS(19),
    [anon_sym_cursor] = ACTIONS(19),
    [anon_sym_smooth_DASHscroll] = ACTIONS(19),
    [anon_sym_whover] = ACTIONS(19),
    [anon_sym_on] = ACTIONS(19),
    [anon_sym_mobile_DASHswipe] = ACTIONS(19),
    [anon_sym_mobile_DASHtext] = ACTIONS(19),
    [anon_sym_reveal] = ACTIONS(19),
    [anon_sym_on_DASHmutation] = ACTIONS(19),
    [anon_sym_ios] = ACTIONS(19),
    [anon_sym_capture_state_report] = ACTIONS(19),
    [anon_sym_program] = ACTIONS(19),
    [anon_sym_media] = ACTIONS(19),
    [anon_sym_snapshot] = ACTIONS(19),
    [anon_sym_test_DASHskip] = ACTIONS(19),
    [anon_sym_fade_DASHin_DASHdown] = ACTIONS(19),
    [anon_sym_eval] = ACTIONS(19),
    [anon_sym_mobile_DASHmotion] = ACTIONS(19),
    [anon_sym_wscroll] = ACTIONS(19),
    [anon_sym_else] = ACTIONS(19),
    [anon_sym_match] = ACTIONS(19),
    [anon_sym_resize] = ACTIONS(19),
    [anon_sym_each] = ACTIONS(19),
    [anon_sym_mobile_DASHinput] = ACTIONS(19),
    [anon_sym_test_list] = ACTIONS(19),
    [anon_sym_after] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHradius] = ACTIONS(19),
    [anon_sym_magnetic_DASHglow] = ACTIONS(19),
    [anon_sym_fill] = ACTIONS(19),
    [anon_sym_import] = ACTIONS(19),
    [anon_sym_particle_DASHfield] = ACTIONS(19),
    [anon_sym_record_DASHtimeline] = ACTIONS(19),
    [anon_sym_assert_DASHmock_DASHcalled] = ACTIONS(19),
    [anon_sym_effect] = ACTIONS(19),
    [anon_sym_video_DASHvisibility] = ACTIONS(19),
    [anon_sym_wloop] = ACTIONS(19),
    [anon_sym_biometric] = ACTIONS(19),
    [anon_sym_touch_DASHpinch] = ACTIONS(19),
    [anon_sym_notification] = ACTIONS(19),
    [anon_sym_sequence] = ACTIONS(19),
    [anon_sym_safe_DASHarea_DASHcontainer] = ACTIONS(19),
    [anon_sym_mobile_DASHtextarea] = ACTIONS(19),
    [anon_sym_breakpoint] = ACTIONS(19),
    [anon_sym_pinch] = ACTIONS(19),
    [anon_sym_touch_DASHstate] = ACTIONS(19),
    [anon_sym_run] = ACTIONS(19),
    [anon_sym_sheet] = ACTIONS(19),
    [anon_sym_richtext] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHintensity] = ACTIONS(19),
    [anon_sym_fn] = ACTIONS(19),
    [anon_sym_on_DASHhover] = ACTIONS(19),
    [anon_sym_scroll] = ACTIONS(19),
    [anon_sym_replace_DASHregex] = ACTIONS(19),
    [anon_sym_enable_timing] = ACTIONS(19),
    [anon_sym_in] = ACTIONS(19),
    [anon_sym_modal] = ACTIONS(19),
    [anon_sym_type] = ACTIONS(19),
    [anon_sym_pulse_DASHfalloff] = ACTIONS(19),
    [anon_sym_tabs] = ACTIONS(19),
    [anon_sym_mobile] = ACTIONS(19),
    [anon_sym_drag] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHcolor] = ACTIONS(19),
    [anon_sym_wait_for_signal] = ACTIONS(19),
    [anon_sym_test] = ACTIONS(19),
    [anon_sym_mock] = ACTIONS(19),
    [anon_sym_on_DASHclick] = ACTIONS(19),
    [anon_sym_cleanup] = ACTIONS(19),
    [anon_sym_timeline] = ACTIONS(19),
    [anon_sym_network_DASHset] = ACTIONS(19),
    [anon_sym_touch_DASHswipe] = ACTIONS(19),
    [anon_sym_include] = ACTIONS(19),
    [anon_sym_mobile_DASHtypography] = ACTIONS(19),
    [anon_sym_model] = ACTIONS(19),
    [anon_sym_react_DASHto_DASHcenter] = ACTIONS(19),
    [anon_sym_wait_for_state] = ACTIONS(19),
    [anon_sym_capture_signal_report] = ACTIONS(19),
    [anon_sym_camera] = ACTIONS(19),
    [anon_sym_device_DASHframe] = ACTIONS(19),
    [anon_sym_mock_DASHnative] = ACTIONS(19),
    [anon_sym_touch_DASHdrag] = ACTIONS(19),
    [anon_sym_mock_DASHpermission] = ACTIONS(19),
    [anon_sym_async_transition] = ACTIONS(19),
    [anon_sym_device] = ACTIONS(19),
    [anon_sym_mobile_DASHbutton] = ACTIONS(19),
    [anon_sym_portal] = ACTIONS(19),
    [anon_sym_mobile_DASHcolors_DASHdark] = ACTIONS(19),
    [anon_sym_click] = ACTIONS(19),
    [anon_sym_touch_DASHtap] = ACTIONS(19),
    [anon_sym_wait] = ACTIONS(19),
    [anon_sym_on_DASHfocus] = ACTIONS(19),
    [anon_sym_fuzz] = ACTIONS(19),
    [anon_sym_output_DASHjson] = ACTIONS(19),
    [anon_sym_offline] = ACTIONS(19),
    [anon_sym_fixture] = ACTIONS(19),
    [anon_sym_on_DASHintersect] = ACTIONS(19),
    [anon_sym_print] = ACTIONS(19),
    [anon_sym_out] = ACTIONS(19),
    [anon_sym_enable_state_tracking] = ACTIONS(19),
    [anon_sym_scroll_DASHview] = ACTIONS(19),
    [anon_sym_native] = ACTIONS(19),
    [anon_sym_scene] = ACTIONS(19),
    [anon_sym_state_machine_block] = ACTIONS(19),
    [anon_sym_hover] = ACTIONS(19),
    [anon_sym_haptic] = ACTIONS(19),
    [anon_sym_editable] = ACTIONS(19),
    [anon_sym_test_DASHonly] = ACTIONS(19),
    [anon_sym_device_DASHcustom] = ACTIONS(19),
    [anon_sym_mobile_DASHtheme] = ACTIONS(19),
    [anon_sym_ws_DASHon] = ACTIONS(19),
    [anon_sym_cart_DASHcount] = ACTIONS(19),
    [anon_sym_cycle] = ACTIONS(19),
    [anon_sym_for] = ACTIONS(19),
    [anon_sym_light] = ACTIONS(19),
    [anon_sym_fade_DASHin_DASHstagger] = ACTIONS(19),
    [anon_sym_morph_DASHto] = ACTIONS(19),
    [anon_sym_channel] = ACTIONS(19),
    [anon_sym_mobile_DASHsearch] = ACTIONS(19),
    [anon_sym_slow_DASHnetwork] = ACTIONS(19),
    [anon_sym_mobile_DASHinput_DASHfield] = ACTIONS(19),
    [anon_sym_if] = ACTIONS(19),
    [anon_sym_use] = ACTIONS(19),
    [anon_sym_mobile_DASHcolors] = ACTIONS(19),
    [anon_sym_script] = ACTIONS(19),
    [anon_sym_mouse_DASHposition] = ACTIONS(19),
    [anon_sym_show] = ACTIONS(19),
    [anon_sym_surface] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHfalloff] = ACTIONS(19),
    [anon_sym_property] = ACTIONS(19),
    [anon_sym_magnetic] = ACTIONS(19),
    [anon_sym_react_DASHto_DASHradius] = ACTIONS(19),
    [anon_sym_safe_DASHarea_DASHinset] = ACTIONS(19),
    [anon_sym_repeat] = ACTIONS(19),
    [anon_sym_chain] = ACTIONS(19),
    [anon_sym_locale] = ACTIONS(19),
    [anon_sym_dark] = ACTIONS(19),
    [anon_sym_compute] = ACTIONS(19),
    [anon_sym_state_machine] = ACTIONS(19),
    [anon_sym_capture_timing_report] = ACTIONS(19),
    [anon_sym_load] = ACTIONS(19),
    [anon_sym_morph_DASHto_DASHglow] = ACTIONS(19),
    [anon_sym_behavior] = ACTIONS(19),
    [anon_sym_share] = ACTIONS(19),
    [anon_sym_cart_DASHtotal] = ACTIONS(19),
    [anon_sym_value_DASHchange] = ACTIONS(19),
    [anon_sym_fade_DASHin] = ACTIONS(19),
    [anon_sym_output] = ACTIONS(19),
    [anon_sym_android] = ACTIONS(19),
    [anon_sym_loop] = ACTIONS(19),
    [anon_sym_portrait] = ACTIONS(19),
    [anon_sym_pulse_DASHradius] = ACTIONS(19),
    [anon_sym_bind] = ACTIONS(19),
    [anon_sym_try] = ACTIONS(19),
    [anon_sym_drive] = ACTIONS(19),
    [anon_sym_log] = ACTIONS(19),
    [anon_sym_scroll_DASHspy] = ACTIONS(19),
    [anon_sym_websocket] = ACTIONS(19),
    [anon_sym_let] = ACTIONS(19),
    [anon_sym_network] = ACTIONS(19),
    [anon_sym_editable_DASHblock] = ACTIONS(19),
    [anon_sym_touch_DASHpreset] = ACTIONS(19),
    [anon_sym_template] = ACTIONS(19),
    [anon_sym_swarm] = ACTIONS(19),
    [anon_sym_persist] = ACTIONS(19),
    [anon_sym_won] = ACTIONS(19),
    [anon_sym_stack] = ACTIONS(19),
    [anon_sym_pulse_DASHintensity] = ACTIONS(19),
    [anon_sym_wclick] = ACTIONS(19),
    [anon_sym_flush] = ACTIONS(19),
    [anon_sym_wait_for_timeline] = ACTIONS(19),
    [anon_sym_safe_DASHarea] = ACTIONS(19),
    [anon_sym_connect] = ACTIONS(19),
    [anon_sym_error_list] = ACTIONS(19),
    [anon_sym_reduced_DASHmotion] = ACTIONS(19),
    [anon_sym_SLASH_SLASH] = ACTIONS(3),
    [anon_sym_SLASH_STAR] = ACTIONS(5),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(31), 1,
      anon_sym_LPAREN,
    ACTIONS(33), 1,
      anon_sym_COLON,
    STATE(5), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(29), 16,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [47] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(6), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(37), 18,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [90] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(39), 1,
      anon_sym_COLON,
    STATE(7), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(29), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [135] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    STATE(33), 1,
      sym_variable_ref,
    STATE(46), 1,
      sym__expression,
    STATE(180), 1,
      sym__arg,
    STATE(8), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(51), 3,
      sym_number,
      sym_percentage,
      sym_color,
    STATE(42), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(49), 4,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
    ACTIONS(41), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [195] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(9), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(55), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [237] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(10), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(57), 4,
      anon_sym_in,
      anon_sym_as,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(63), 6,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(61), 8,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
    ACTIONS(59), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_DOLLAR,
      sym_selector,
  [283] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(67), 1,
      sym_property_name,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(164), 1,
      sym__arg,
    ACTIONS(71), 2,
      sym_string,
      sym_template_string,
    STATE(11), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(73), 5,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(41), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [343] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(12), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(75), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(77), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [385] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(13), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(79), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(81), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [427] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(14), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(83), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(85), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [469] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(15), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(87), 4,
      anon_sym_in,
      anon_sym_as,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(63), 6,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(61), 8,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
    ACTIONS(89), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_DOLLAR,
      sym_selector,
  [515] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(16), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(91), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(93), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [557] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(67), 1,
      sym_property_name,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(164), 1,
      sym__arg,
    STATE(17), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(71), 3,
      sym_string,
      sym_template_string,
      sym_color,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(73), 4,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(41), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [621] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(31), 1,
      anon_sym_LPAREN,
    STATE(18), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(29), 16,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [665] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(101), 1,
      sym_property_name,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(164), 1,
      sym__arg,
    STATE(19), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(71), 3,
      sym_string,
      sym_template_string,
      sym_color,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(73), 4,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(99), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [729] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(20), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(29), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [771] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    STATE(33), 1,
      sym_variable_ref,
    STATE(46), 1,
      sym__expression,
    STATE(180), 1,
      sym__arg,
    STATE(21), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(51), 3,
      sym_number,
      sym_percentage,
      sym_color,
    STATE(42), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(49), 4,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
    ACTIONS(99), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [831] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(101), 1,
      sym_property_name,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(164), 1,
      sym__arg,
    ACTIONS(71), 2,
      sym_string,
      sym_template_string,
    STATE(22), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(73), 5,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(99), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [891] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(23), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(105), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [933] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(24), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(107), 4,
      anon_sym_in,
      anon_sym_as,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(63), 6,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(61), 8,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
    ACTIONS(109), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_DOLLAR,
      sym_selector,
  [979] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(25), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(111), 10,
      anon_sym_in,
      anon_sym_as,
      anon_sym_DASH,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      anon_sym_AMP,
      sym_property_name,
    ACTIONS(113), 17,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_PLUS,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [1021] = 20,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(115), 1,
      sym_identifier,
    ACTIONS(121), 1,
      anon_sym_SEMI,
    ACTIONS(123), 1,
      anon_sym_LPAREN,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    ACTIONS(127), 1,
      anon_sym_AMP,
    ACTIONS(129), 1,
      sym_property_name,
    ACTIONS(131), 1,
      anon_sym_DOLLAR,
    ACTIONS(133), 1,
      anon_sym_TILDE,
    STATE(48), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(84), 1,
      sym__inline_arg,
    STATE(182), 1,
      sym__directive_args,
    STATE(184), 1,
      sym__inline_directive_args,
    STATE(192), 1,
      sym_block,
    ACTIONS(135), 2,
      sym_string,
      sym_template_string,
    STATE(26), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(119), 3,
      anon_sym_AT,
      anon_sym_RBRACE,
      sym_selector,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(117), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
  [1092] = 20,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(115), 1,
      sym_identifier,
    ACTIONS(121), 1,
      anon_sym_SEMI,
    ACTIONS(123), 1,
      anon_sym_LPAREN,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    ACTIONS(127), 1,
      anon_sym_AMP,
    ACTIONS(129), 1,
      sym_property_name,
    ACTIONS(131), 1,
      anon_sym_DOLLAR,
    ACTIONS(133), 1,
      anon_sym_TILDE,
    STATE(49), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(84), 1,
      sym__inline_arg,
    STATE(182), 1,
      sym__directive_args,
    STATE(184), 1,
      sym__inline_directive_args,
    STATE(192), 1,
      sym_block,
    ACTIONS(135), 2,
      sym_string,
      sym_template_string,
    STATE(27), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(119), 3,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(117), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
  [1163] = 19,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(139), 1,
      sym_identifier,
    ACTIONS(143), 1,
      anon_sym_SEMI,
    ACTIONS(145), 1,
      anon_sym_LPAREN,
    ACTIONS(147), 1,
      anon_sym_LBRACE,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(153), 1,
      anon_sym_TILDE,
    STATE(50), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(98), 1,
      sym__inline_arg,
    STATE(197), 1,
      sym__directive_args,
    STATE(201), 1,
      sym__inline_directive_args,
    STATE(219), 1,
      sym_block,
    STATE(28), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(141), 3,
      anon_sym_in,
      anon_sym_as,
      sym_number,
    STATE(107), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(137), 4,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_selector,
    ACTIONS(155), 4,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [1232] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(157), 1,
      anon_sym_LPAREN,
    ACTIONS(159), 1,
      anon_sym_COLON,
    STATE(29), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(29), 16,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1277] = 20,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(115), 1,
      sym_identifier,
    ACTIONS(123), 1,
      anon_sym_LPAREN,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    ACTIONS(127), 1,
      anon_sym_AMP,
    ACTIONS(131), 1,
      anon_sym_DOLLAR,
    ACTIONS(133), 1,
      anon_sym_TILDE,
    ACTIONS(161), 1,
      anon_sym_SEMI,
    ACTIONS(163), 1,
      sym_property_name,
    STATE(49), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(84), 1,
      sym__inline_arg,
    STATE(184), 1,
      sym__inline_directive_args,
    STATE(185), 1,
      sym__directive_args,
    STATE(199), 1,
      sym_block,
    ACTIONS(135), 2,
      sym_string,
      sym_template_string,
    STATE(30), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(137), 3,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(117), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
  [1348] = 20,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(115), 1,
      sym_identifier,
    ACTIONS(123), 1,
      anon_sym_LPAREN,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    ACTIONS(127), 1,
      anon_sym_AMP,
    ACTIONS(131), 1,
      anon_sym_DOLLAR,
    ACTIONS(133), 1,
      anon_sym_TILDE,
    ACTIONS(161), 1,
      anon_sym_SEMI,
    ACTIONS(163), 1,
      sym_property_name,
    STATE(48), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(84), 1,
      sym__inline_arg,
    STATE(184), 1,
      sym__inline_directive_args,
    STATE(185), 1,
      sym__directive_args,
    STATE(199), 1,
      sym_block,
    ACTIONS(135), 2,
      sym_string,
      sym_template_string,
    STATE(31), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(137), 3,
      anon_sym_AT,
      anon_sym_RBRACE,
      sym_selector,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(117), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
  [1419] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(32), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(37), 18,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1460] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(165), 1,
      anon_sym_COLON,
    STATE(33), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(29), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1503] = 19,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(139), 1,
      sym_identifier,
    ACTIONS(145), 1,
      anon_sym_LPAREN,
    ACTIONS(147), 1,
      anon_sym_LBRACE,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(153), 1,
      anon_sym_TILDE,
    ACTIONS(167), 1,
      anon_sym_SEMI,
    STATE(50), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(98), 1,
      sym__inline_arg,
    STATE(201), 1,
      sym__inline_directive_args,
    STATE(202), 1,
      sym__directive_args,
    STATE(229), 1,
      sym_block,
    STATE(34), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(141), 3,
      anon_sym_in,
      anon_sym_as,
      sym_number,
    STATE(107), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(119), 4,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_selector,
    ACTIONS(155), 4,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [1572] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(35), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(105), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1612] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(36), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(111), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(113), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1652] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(157), 1,
      anon_sym_LPAREN,
    STATE(37), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(29), 16,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1694] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(169), 1,
      sym_identifier,
    ACTIONS(177), 1,
      anon_sym_AMP,
    ACTIONS(180), 1,
      sym_property_name,
    ACTIONS(182), 1,
      anon_sym_DOLLAR,
    ACTIONS(185), 1,
      anon_sym_TILDE,
    STATE(84), 1,
      sym__inline_arg,
    ACTIONS(188), 2,
      sym_string,
      sym_template_string,
    STATE(38), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__inline_directive_args_repeat1,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(172), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(175), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      sym_selector,
  [1750] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(39), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(55), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1790] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(40), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(87), 3,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
    ACTIONS(193), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(89), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
    ACTIONS(191), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [1834] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(41), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(91), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(93), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1874] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(42), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(29), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1914] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(43), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(75), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(77), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1954] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(44), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(79), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(81), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [1994] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(45), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(83), 8,
      anon_sym_in,
      anon_sym_as,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(85), 17,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      sym_selector,
  [2034] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(46), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(57), 3,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
    ACTIONS(193), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(59), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
    ACTIONS(191), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [2078] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(47), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(107), 3,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
    ACTIONS(193), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(109), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
    ACTIONS(191), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [2122] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(115), 1,
      sym_identifier,
    ACTIONS(127), 1,
      anon_sym_AMP,
    ACTIONS(131), 1,
      anon_sym_DOLLAR,
    ACTIONS(133), 1,
      anon_sym_TILDE,
    ACTIONS(197), 1,
      anon_sym_COLON,
    ACTIONS(199), 1,
      sym_property_name,
    STATE(38), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(84), 1,
      sym__inline_arg,
    ACTIONS(135), 2,
      sym_string,
      sym_template_string,
    STATE(48), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(117), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(195), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      sym_selector,
  [2181] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(115), 1,
      sym_identifier,
    ACTIONS(127), 1,
      anon_sym_AMP,
    ACTIONS(131), 1,
      anon_sym_DOLLAR,
    ACTIONS(133), 1,
      anon_sym_TILDE,
    ACTIONS(199), 1,
      sym_property_name,
    ACTIONS(201), 1,
      anon_sym_COLON,
    STATE(38), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(84), 1,
      sym__inline_arg,
    ACTIONS(135), 2,
      sym_string,
      sym_template_string,
    STATE(49), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(96), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(117), 5,
      anon_sym_in,
      anon_sym_as,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(195), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [2240] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(139), 1,
      sym_identifier,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(153), 1,
      anon_sym_TILDE,
    ACTIONS(203), 1,
      anon_sym_COLON,
    STATE(51), 1,
      aux_sym__inline_directive_args_repeat1,
    STATE(98), 1,
      sym__inline_arg,
    STATE(50), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(141), 3,
      anon_sym_in,
      anon_sym_as,
      sym_number,
    STATE(107), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(155), 4,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
    ACTIONS(195), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [2297] = 12,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(205), 1,
      sym_identifier,
    ACTIONS(211), 1,
      anon_sym_AMP,
    ACTIONS(214), 1,
      anon_sym_DOLLAR,
    ACTIONS(217), 1,
      anon_sym_TILDE,
    STATE(98), 1,
      sym__inline_arg,
    ACTIONS(208), 3,
      anon_sym_in,
      anon_sym_as,
      sym_number,
    STATE(51), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__inline_directive_args_repeat1,
    STATE(107), 3,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(220), 4,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
    ACTIONS(175), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [2350] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(227), 1,
      anon_sym_RPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(322), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(52), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2412] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(239), 1,
      sym_identifier,
    ACTIONS(243), 1,
      sym_property_name,
    ACTIONS(245), 1,
      anon_sym_TILDE,
    STATE(54), 1,
      aux_sym__property_value_repeat1,
    STATE(131), 1,
      sym__value,
    STATE(148), 1,
      sym_transition_arrow,
    ACTIONS(247), 2,
      sym_string,
      sym_template_string,
    STATE(53), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(134), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(249), 5,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(241), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [2466] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(251), 1,
      sym_identifier,
    ACTIONS(256), 1,
      anon_sym_AMP,
    ACTIONS(259), 1,
      sym_property_name,
    ACTIONS(261), 1,
      anon_sym_DOLLAR,
    ACTIONS(264), 1,
      anon_sym_TILDE,
    STATE(131), 1,
      sym__value,
    STATE(148), 1,
      sym_transition_arrow,
    ACTIONS(267), 2,
      sym_string,
      sym_template_string,
    STATE(54), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__property_value_repeat1,
    ACTIONS(254), 4,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      sym_selector,
    STATE(134), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(270), 5,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
  [2522] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(273), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(303), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(55), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2584] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(243), 1,
      sym_property_name,
    ACTIONS(275), 1,
      sym_identifier,
    ACTIONS(277), 1,
      anon_sym_AMP,
    ACTIONS(279), 1,
      anon_sym_DOLLAR,
    ACTIONS(281), 1,
      anon_sym_TILDE,
    STATE(64), 1,
      aux_sym__property_value_repeat1,
    STATE(140), 1,
      sym__value,
    STATE(149), 1,
      sym_transition_arrow,
    STATE(56), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(283), 3,
      sym_string,
      sym_template_string,
      sym_color,
    ACTIONS(241), 4,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
    ACTIONS(285), 4,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    STATE(133), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
  [2642] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(287), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(316), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(57), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2704] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(289), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(328), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(58), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2766] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(291), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(323), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(59), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2828] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(293), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(318), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(60), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2890] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(61), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 7,
      anon_sym_in,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
      sym_identifier,
    ACTIONS(37), 16,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_RPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
      anon_sym_DOLLAR,
      sym_selector,
  [2928] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(295), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(300), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(62), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [2990] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(297), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(307), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(63), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3052] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(259), 1,
      sym_property_name,
    ACTIONS(299), 1,
      sym_identifier,
    ACTIONS(302), 1,
      anon_sym_AMP,
    ACTIONS(305), 1,
      anon_sym_DOLLAR,
    ACTIONS(308), 1,
      anon_sym_TILDE,
    STATE(140), 1,
      sym__value,
    STATE(149), 1,
      sym_transition_arrow,
    ACTIONS(311), 3,
      sym_string,
      sym_template_string,
      sym_color,
    STATE(64), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__property_value_repeat1,
    ACTIONS(254), 4,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
    ACTIONS(314), 4,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    STATE(133), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
  [3108] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(317), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(302), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(65), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3170] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(319), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(306), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(66), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3232] = 17,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(321), 1,
      anon_sym_RPAREN,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(263), 1,
      sym__arg,
    STATE(299), 1,
      sym__args_inner,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(67), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3294] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(41), 1,
      anon_sym_RPAREN,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(269), 1,
      sym__arg,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(68), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3353] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(323), 1,
      anon_sym_LPAREN,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(181), 1,
      sym__arg,
    STATE(183), 1,
      sym__args_inner,
    ACTIONS(73), 2,
      sym_number,
      sym_percentage,
    STATE(69), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(71), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3412] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(99), 1,
      anon_sym_RPAREN,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(269), 1,
      sym__arg,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(70), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3471] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(323), 1,
      anon_sym_LPAREN,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(167), 1,
      sym__arg,
    STATE(183), 1,
      sym__args_inner,
    ACTIONS(73), 2,
      sym_number,
      sym_percentage,
    STATE(71), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(71), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3530] = 16,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    ACTIONS(325), 1,
      sym_identifier,
    ACTIONS(327), 1,
      anon_sym_LPAREN,
    STATE(33), 1,
      sym_variable_ref,
    STATE(46), 1,
      sym__expression,
    STATE(174), 1,
      sym__arg,
    STATE(195), 1,
      sym__args_inner,
    ACTIONS(51), 2,
      sym_number,
      sym_percentage,
    STATE(72), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(42), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(49), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3589] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    ACTIONS(325), 1,
      sym_identifier,
    ACTIONS(327), 1,
      anon_sym_LPAREN,
    STATE(33), 1,
      sym_variable_ref,
    STATE(46), 1,
      sym__expression,
    STATE(180), 1,
      sym__arg,
    ACTIONS(51), 2,
      sym_number,
      sym_percentage,
    STATE(73), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(42), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(49), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3645] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    STATE(100), 1,
      sym_variable_ref,
    STATE(118), 1,
      sym__expression,
    STATE(269), 1,
      sym__arg,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(74), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3701] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(65), 1,
      sym_identifier,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(323), 1,
      anon_sym_LPAREN,
    STATE(7), 1,
      sym_variable_ref,
    STATE(10), 1,
      sym__expression,
    STATE(164), 1,
      sym__arg,
    ACTIONS(73), 2,
      sym_number,
      sym_percentage,
    STATE(75), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 3,
      sym_function_call,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(71), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3757] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(329), 1,
      anon_sym_LPAREN,
    STATE(76), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(29), 14,
      anon_sym_in,
      anon_sym_SEMI,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [3794] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    STATE(111), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(77), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3845] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    STATE(111), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(78), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3896] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    STATE(151), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(79), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [3947] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(80), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(105), 13,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [3982] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    STATE(117), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(81), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4033] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(82), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(37), 13,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [4068] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(83), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(55), 13,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [4103] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(84), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(335), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(337), 13,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [4138] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(323), 1,
      anon_sym_LPAREN,
    ACTIONS(339), 1,
      sym_identifier,
    STATE(24), 1,
      sym__expression,
    ACTIONS(73), 2,
      sym_number,
      sym_percentage,
    STATE(85), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(71), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4189] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(329), 1,
      anon_sym_LPAREN,
    ACTIONS(341), 1,
      anon_sym_COLON,
    STATE(86), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(29), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [4228] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    ACTIONS(327), 1,
      anon_sym_LPAREN,
    ACTIONS(343), 1,
      sym_identifier,
    STATE(36), 1,
      sym__expression,
    ACTIONS(51), 2,
      sym_number,
      sym_percentage,
    STATE(87), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(42), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(49), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4279] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    STATE(150), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(88), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4330] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    ACTIONS(327), 1,
      anon_sym_LPAREN,
    ACTIONS(343), 1,
      sym_identifier,
    STATE(47), 1,
      sym__expression,
    ACTIONS(51), 2,
      sym_number,
      sym_percentage,
    STATE(89), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(42), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(49), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4381] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(43), 1,
      anon_sym_AMP,
    ACTIONS(45), 1,
      anon_sym_DOLLAR,
    ACTIONS(47), 1,
      anon_sym_TILDE,
    ACTIONS(327), 1,
      anon_sym_LPAREN,
    ACTIONS(343), 1,
      sym_identifier,
    STATE(40), 1,
      sym__expression,
    ACTIONS(51), 2,
      sym_number,
      sym_percentage,
    STATE(90), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(43), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(42), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(49), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4432] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    STATE(152), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(91), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4483] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(323), 1,
      anon_sym_LPAREN,
    ACTIONS(339), 1,
      sym_identifier,
    STATE(15), 1,
      sym__expression,
    ACTIONS(73), 2,
      sym_number,
      sym_percentage,
    STATE(92), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(71), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4534] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(93), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(345), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(347), 13,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [4569] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(225), 1,
      anon_sym_LPAREN,
    ACTIONS(229), 1,
      anon_sym_AMP,
    ACTIONS(231), 1,
      anon_sym_DOLLAR,
    ACTIONS(233), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    STATE(112), 1,
      sym__expression,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(94), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(116), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4620] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(69), 1,
      anon_sym_TILDE,
    ACTIONS(95), 1,
      anon_sym_AMP,
    ACTIONS(97), 1,
      anon_sym_DOLLAR,
    ACTIONS(323), 1,
      anon_sym_LPAREN,
    ACTIONS(339), 1,
      sym_identifier,
    STATE(25), 1,
      sym__expression,
    ACTIONS(73), 2,
      sym_number,
      sym_percentage,
    STATE(95), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(12), 3,
      sym__value,
      sym_binary_expr,
      sym_paren_expr,
    STATE(20), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(71), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4671] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(96), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(345), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(347), 13,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [4706] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(97), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(345), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(347), 15,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [4740] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(98), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(335), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(337), 15,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [4774] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(99), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(349), 7,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
    ACTIONS(351), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [4808] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(353), 1,
      anon_sym_COLON,
    STATE(100), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(29), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [4844] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(239), 1,
      sym_identifier,
    ACTIONS(245), 1,
      anon_sym_TILDE,
    ACTIONS(355), 1,
      anon_sym_AMP,
    ACTIONS(357), 1,
      anon_sym_DOLLAR,
    STATE(53), 1,
      aux_sym__property_value_repeat1,
    STATE(131), 1,
      sym__value,
    STATE(148), 1,
      sym_transition_arrow,
    STATE(194), 1,
      sym__property_value,
    ACTIONS(249), 2,
      sym_number,
      sym_percentage,
    STATE(101), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(134), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(247), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4896] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(102), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(79), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(81), 14,
      anon_sym_in,
      anon_sym_SEMI,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [4930] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(275), 1,
      sym_identifier,
    ACTIONS(277), 1,
      anon_sym_AMP,
    ACTIONS(279), 1,
      anon_sym_DOLLAR,
    ACTIONS(281), 1,
      anon_sym_TILDE,
    STATE(56), 1,
      aux_sym__property_value_repeat1,
    STATE(140), 1,
      sym__value,
    STATE(149), 1,
      sym_transition_arrow,
    STATE(194), 1,
      sym__property_value,
    ACTIONS(285), 2,
      sym_number,
      sym_percentage,
    STATE(103), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(133), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(283), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [4982] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(104), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(29), 14,
      anon_sym_in,
      anon_sym_SEMI,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5016] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(105), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(91), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(93), 14,
      anon_sym_in,
      anon_sym_SEMI,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5050] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(106), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(37), 14,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COLON,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5084] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(107), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(345), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(347), 15,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [5118] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(108), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(55), 15,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [5152] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(109), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(37), 15,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [5186] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(110), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(105), 15,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COLON,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [5220] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(111), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(111), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(113), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5253] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(112), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(109), 4,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
    ACTIONS(361), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(359), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5288] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(113), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(349), 4,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
      sym_number,
    ACTIONS(351), 14,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
  [5321] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(114), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(55), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5354] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(115), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(105), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5387] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(116), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(75), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(77), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5420] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(117), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(89), 4,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
    ACTIONS(361), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(359), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5455] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(118), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(59), 4,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
    ACTIONS(361), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(359), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5490] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(119), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(83), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(85), 13,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [5523] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(363), 1,
      anon_sym_LPAREN,
    STATE(120), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(29), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [5558] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(365), 1,
      anon_sym_LPAREN,
    STATE(121), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(29), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [5593] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(122), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(55), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [5625] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(367), 1,
      anon_sym_AT,
    ACTIONS(369), 1,
      anon_sym_RBRACE,
    ACTIONS(371), 1,
      sym_property_name,
    STATE(128), 1,
      aux_sym_block_repeat1,
    STATE(226), 1,
      sym__block_item,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(292), 1,
      sym_variable_ref,
    STATE(123), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(286), 2,
      sym_selector_list,
      sym_element_ref,
    STATE(228), 5,
      sym_directive,
      sym_generic_directive,
      sym_nested_scope,
      sym_property_decl,
      sym_value_decl,
  [5677] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(367), 1,
      anon_sym_AT,
    ACTIONS(371), 1,
      sym_property_name,
    ACTIONS(373), 1,
      anon_sym_RBRACE,
    STATE(123), 1,
      aux_sym_block_repeat1,
    STATE(226), 1,
      sym__block_item,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(292), 1,
      sym_variable_ref,
    STATE(124), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(286), 2,
      sym_selector_list,
      sym_element_ref,
    STATE(228), 5,
      sym_directive,
      sym_generic_directive,
      sym_nested_scope,
      sym_property_decl,
      sym_value_decl,
  [5729] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(125), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(37), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [5761] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(126), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(105), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [5793] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(127), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(37), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [5825] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(375), 1,
      anon_sym_AT,
    ACTIONS(378), 1,
      anon_sym_RBRACE,
    ACTIONS(380), 1,
      anon_sym_AMP,
    ACTIONS(383), 1,
      sym_property_name,
    ACTIONS(386), 1,
      anon_sym_DOLLAR,
    ACTIONS(389), 1,
      sym_selector,
    STATE(226), 1,
      sym__block_item,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(292), 1,
      sym_variable_ref,
    STATE(286), 2,
      sym_selector_list,
      sym_element_ref,
    STATE(128), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym_block_repeat1,
    STATE(228), 5,
      sym_directive,
      sym_generic_directive,
      sym_nested_scope,
      sym_property_decl,
      sym_value_decl,
  [5875] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(129), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(79), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(81), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [5907] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(130), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(53), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(55), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [5939] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(394), 1,
      anon_sym_DASH_GT,
    STATE(131), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(396), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(392), 10,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [5973] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(132), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(91), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(93), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [6005] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(133), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(29), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6037] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(134), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(27), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(29), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [6069] = 12,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(398), 1,
      ts_builtin_sym_end,
    ACTIONS(400), 1,
      sym_identifier,
    ACTIONS(403), 1,
      anon_sym_AT,
    ACTIONS(406), 1,
      anon_sym_PERCENT,
    ACTIONS(409), 1,
      sym_selector,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(258), 1,
      sym__item,
    STATE(287), 1,
      sym_selector_list,
    STATE(135), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym_source_file_repeat1,
    STATE(256), 8,
      sym_directive,
      sym_generic_directive,
      sym_emit_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_scope_block,
  [6115] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(136), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(103), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(105), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6147] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(367), 1,
      anon_sym_AT,
    ACTIONS(371), 1,
      sym_property_name,
    ACTIONS(412), 1,
      anon_sym_RBRACE,
    STATE(141), 1,
      aux_sym_block_repeat1,
    STATE(226), 1,
      sym__block_item,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(292), 1,
      sym_variable_ref,
    STATE(137), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(286), 2,
      sym_selector_list,
      sym_element_ref,
    STATE(228), 5,
      sym_directive,
      sym_generic_directive,
      sym_nested_scope,
      sym_property_decl,
      sym_value_decl,
  [6199] = 13,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(9), 1,
      sym_identifier,
    ACTIONS(11), 1,
      anon_sym_AT,
    ACTIONS(13), 1,
      anon_sym_PERCENT,
    ACTIONS(15), 1,
      sym_selector,
    ACTIONS(414), 1,
      ts_builtin_sym_end,
    STATE(135), 1,
      aux_sym_source_file_repeat1,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(258), 1,
      sym__item,
    STATE(287), 1,
      sym_selector_list,
    STATE(138), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(256), 8,
      sym_directive,
      sym_generic_directive,
      sym_emit_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_scope_block,
  [6247] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(139), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(79), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(81), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6279] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(416), 1,
      anon_sym_DASH_GT,
    STATE(140), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(396), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(392), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6313] = 15,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(367), 1,
      anon_sym_AT,
    ACTIONS(371), 1,
      sym_property_name,
    ACTIONS(418), 1,
      anon_sym_RBRACE,
    STATE(128), 1,
      aux_sym_block_repeat1,
    STATE(226), 1,
      sym__block_item,
    STATE(243), 1,
      aux_sym_selector_list_repeat1,
    STATE(292), 1,
      sym_variable_ref,
    STATE(141), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(286), 2,
      sym_selector_list,
      sym_element_ref,
    STATE(228), 5,
      sym_directive,
      sym_generic_directive,
      sym_nested_scope,
      sym_property_decl,
      sym_value_decl,
  [6365] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(142), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(91), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(93), 12,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_DASH_GT,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6397] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(143), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(422), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(420), 10,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [6428] = 11,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(149), 1,
      anon_sym_AMP,
    ACTIONS(151), 1,
      anon_sym_DOLLAR,
    ACTIONS(153), 1,
      anon_sym_TILDE,
    ACTIONS(331), 1,
      sym_identifier,
    STATE(312), 1,
      sym__value,
    ACTIONS(237), 2,
      sym_number,
      sym_percentage,
    STATE(144), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(104), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(235), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [6471] = 11,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(275), 1,
      sym_identifier,
    ACTIONS(277), 1,
      anon_sym_AMP,
    ACTIONS(279), 1,
      anon_sym_DOLLAR,
    ACTIONS(281), 1,
      anon_sym_TILDE,
    STATE(146), 1,
      sym__value,
    ACTIONS(285), 2,
      sym_number,
      sym_percentage,
    STATE(145), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(133), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(283), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [6514] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(146), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(422), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(420), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6545] = 11,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(239), 1,
      sym_identifier,
    ACTIONS(245), 1,
      anon_sym_TILDE,
    ACTIONS(355), 1,
      anon_sym_AMP,
    ACTIONS(357), 1,
      anon_sym_DOLLAR,
    STATE(143), 1,
      sym__value,
    ACTIONS(249), 2,
      sym_number,
      sym_percentage,
    STATE(147), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(134), 4,
      sym_function_call,
      sym_variable_ref,
      sym_element_ref,
      sym_preset_ref,
    ACTIONS(247), 5,
      sym_string,
      sym_template_string,
      sym_duration,
      sym_dimension,
      sym_color,
  [6588] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(148), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(396), 6,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
      sym_color,
    ACTIONS(392), 10,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_selector,
      sym_string,
      sym_template_string,
  [6619] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(149), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(396), 5,
      sym_property_name,
      sym_number,
      sym_duration,
      sym_dimension,
      sym_percentage,
    ACTIONS(392), 11,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_identifier,
      anon_sym_DOLLAR,
      anon_sym_TILDE,
      sym_string,
      sym_template_string,
      sym_color,
  [6650] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(424), 1,
      anon_sym_RPAREN,
    STATE(150), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(428), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(426), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [6682] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(430), 1,
      anon_sym_RPAREN,
    STATE(151), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(428), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(426), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [6714] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(432), 1,
      anon_sym_RPAREN,
    STATE(152), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(428), 5,
      anon_sym_SLASH,
      anon_sym_EQ_EQ,
      anon_sym_BANG_EQ,
      anon_sym_LT,
      anon_sym_GT,
    ACTIONS(426), 9,
      anon_sym_PLUS,
      anon_sym_DASH,
      anon_sym_STAR,
      anon_sym_EQ_EQ_EQ,
      anon_sym_BANG_EQ_EQ,
      anon_sym_LT_EQ,
      anon_sym_GT_EQ,
      anon_sym_AMP_AMP,
      anon_sym_PIPE_PIPE,
  [6746] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(439), 1,
      anon_sym_COMMA,
    ACTIONS(442), 1,
      sym_property_name,
    ACTIONS(434), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(153), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__args_inner_repeat1,
    ACTIONS(437), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [6779] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(448), 1,
      anon_sym_LPAREN,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    ACTIONS(452), 1,
      sym_property_name,
    ACTIONS(454), 1,
      anon_sym_DOLLAR,
    STATE(177), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(189), 1,
      sym__meta_inline_arg,
    STATE(193), 1,
      sym_variable_ref,
    STATE(227), 1,
      sym__meta_inline_args,
    STATE(279), 1,
      sym_meta_block,
    ACTIONS(444), 2,
      anon_sym_in,
      sym_identifier,
    STATE(154), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(446), 3,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
  [6826] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(448), 1,
      anon_sym_LPAREN,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    ACTIONS(454), 1,
      anon_sym_DOLLAR,
    ACTIONS(458), 1,
      sym_property_name,
    STATE(177), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(189), 1,
      sym__meta_inline_arg,
    STATE(193), 1,
      sym_variable_ref,
    STATE(221), 1,
      sym__meta_inline_args,
    STATE(278), 1,
      sym_meta_block,
    ACTIONS(444), 2,
      anon_sym_in,
      sym_identifier,
    STATE(155), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(456), 3,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
  [6873] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    ACTIONS(460), 1,
      sym_identifier,
    ACTIONS(463), 1,
      anon_sym_in,
    ACTIONS(465), 1,
      anon_sym_LPAREN,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    STATE(176), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(190), 1,
      sym__meta_inline_arg,
    STATE(204), 1,
      sym_variable_ref,
    STATE(210), 1,
      sym__meta_inline_args,
    STATE(255), 1,
      sym_meta_block,
    STATE(156), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(446), 4,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_selector,
  [6920] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    ACTIONS(463), 1,
      anon_sym_in,
    ACTIONS(465), 1,
      anon_sym_LPAREN,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    ACTIONS(471), 1,
      sym_identifier,
    STATE(176), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(190), 1,
      sym__meta_inline_arg,
    STATE(204), 1,
      sym_variable_ref,
    STATE(213), 1,
      sym__meta_inline_args,
    STATE(257), 1,
      sym_meta_block,
    STATE(157), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(469), 4,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_selector,
  [6967] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(448), 1,
      anon_sym_LPAREN,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    ACTIONS(454), 1,
      anon_sym_DOLLAR,
    ACTIONS(474), 1,
      sym_property_name,
    STATE(177), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(189), 1,
      sym__meta_inline_arg,
    STATE(193), 1,
      sym_variable_ref,
    STATE(223), 1,
      sym__meta_inline_args,
    STATE(280), 1,
      sym_meta_block,
    ACTIONS(444), 2,
      anon_sym_in,
      sym_identifier,
    STATE(158), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(469), 3,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
  [7014] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(448), 1,
      anon_sym_LPAREN,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    ACTIONS(454), 1,
      anon_sym_DOLLAR,
    ACTIONS(478), 1,
      sym_property_name,
    STATE(177), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(189), 1,
      sym__meta_inline_arg,
    STATE(193), 1,
      sym_variable_ref,
    STATE(234), 1,
      sym__meta_inline_args,
    STATE(273), 1,
      sym_meta_block,
    ACTIONS(444), 2,
      anon_sym_in,
      sym_identifier,
    STATE(159), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(476), 3,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
  [7061] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(480), 1,
      anon_sym_EQ,
    STATE(160), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(87), 3,
      anon_sym_in,
      anon_sym_as,
      sym_property_name,
    ACTIONS(89), 10,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [7092] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    ACTIONS(463), 1,
      anon_sym_in,
    ACTIONS(465), 1,
      anon_sym_LPAREN,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    ACTIONS(482), 1,
      sym_identifier,
    STATE(176), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(190), 1,
      sym__meta_inline_arg,
    STATE(204), 1,
      sym_variable_ref,
    STATE(209), 1,
      sym__meta_inline_args,
    STATE(253), 1,
      sym_meta_block,
    STATE(161), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(456), 4,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_selector,
  [7139] = 14,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    ACTIONS(463), 1,
      anon_sym_in,
    ACTIONS(465), 1,
      anon_sym_LPAREN,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    ACTIONS(485), 1,
      sym_identifier,
    STATE(176), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(190), 1,
      sym__meta_inline_arg,
    STATE(204), 1,
      sym_variable_ref,
    STATE(208), 1,
      sym__meta_inline_args,
    STATE(247), 1,
      sym_meta_block,
    STATE(162), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(476), 4,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_selector,
  [7186] = 10,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(488), 1,
      anon_sym_AT,
    ACTIONS(490), 1,
      anon_sym_PERCENT,
    ACTIONS(492), 1,
      anon_sym_RBRACE,
    ACTIONS(494), 1,
      sym_property_name,
    STATE(169), 1,
      aux_sym_meta_block_repeat1,
    STATE(277), 1,
      sym__meta_item,
    STATE(163), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(266), 7,
      sym_directive,
      sym_generic_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_property_decl,
  [7224] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(164), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(442), 3,
      anon_sym_in,
      anon_sym_as,
      sym_property_name,
    ACTIONS(437), 10,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [7252] = 10,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(488), 1,
      anon_sym_AT,
    ACTIONS(490), 1,
      anon_sym_PERCENT,
    ACTIONS(494), 1,
      sym_property_name,
    ACTIONS(496), 1,
      anon_sym_RBRACE,
    STATE(166), 1,
      aux_sym_meta_block_repeat1,
    STATE(277), 1,
      sym__meta_item,
    STATE(165), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(266), 7,
      sym_directive,
      sym_generic_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_property_decl,
  [7290] = 9,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(498), 1,
      anon_sym_AT,
    ACTIONS(501), 1,
      anon_sym_PERCENT,
    ACTIONS(504), 1,
      anon_sym_RBRACE,
    ACTIONS(506), 1,
      sym_property_name,
    STATE(277), 1,
      sym__meta_item,
    STATE(166), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym_meta_block_repeat1,
    STATE(266), 7,
      sym_directive,
      sym_generic_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_property_decl,
  [7326] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(513), 1,
      anon_sym_COMMA,
    ACTIONS(515), 1,
      sym_property_name,
    STATE(170), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(509), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(167), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(511), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [7360] = 10,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(488), 1,
      anon_sym_AT,
    ACTIONS(490), 1,
      anon_sym_PERCENT,
    ACTIONS(494), 1,
      sym_property_name,
    ACTIONS(517), 1,
      anon_sym_RBRACE,
    STATE(165), 1,
      aux_sym_meta_block_repeat1,
    STATE(277), 1,
      sym__meta_item,
    STATE(168), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(266), 7,
      sym_directive,
      sym_generic_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_property_decl,
  [7398] = 10,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(488), 1,
      anon_sym_AT,
    ACTIONS(490), 1,
      anon_sym_PERCENT,
    ACTIONS(494), 1,
      sym_property_name,
    ACTIONS(519), 1,
      anon_sym_RBRACE,
    STATE(166), 1,
      aux_sym_meta_block_repeat1,
    STATE(277), 1,
      sym__meta_item,
    STATE(169), 2,
      sym_line_comment,
      sym_block_comment,
    STATE(266), 7,
      sym_directive,
      sym_generic_directive,
      sym_macro_def,
      sym_primitive_def,
      sym_capture_type_def,
      sym_meta_directive,
      sym_property_decl,
  [7436] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(67), 1,
      sym_property_name,
    ACTIONS(521), 1,
      anon_sym_COMMA,
    STATE(153), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(509), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(170), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(41), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      anon_sym_DOLLAR,
      sym_selector,
  [7470] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(67), 1,
      sym_identifier,
    ACTIONS(525), 1,
      anon_sym_COMMA,
    STATE(172), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(523), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(171), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(41), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7503] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(442), 1,
      sym_identifier,
    ACTIONS(530), 1,
      anon_sym_COMMA,
    ACTIONS(527), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(172), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__args_inner_repeat1,
    ACTIONS(437), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7534] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(533), 1,
      anon_sym_EQ,
    STATE(173), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(87), 3,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
    ACTIONS(89), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7563] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(515), 1,
      sym_identifier,
    ACTIONS(535), 1,
      anon_sym_COMMA,
    STATE(171), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(523), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(174), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(511), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7596] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(542), 1,
      anon_sym_DOLLAR,
    STATE(190), 1,
      sym__meta_inline_arg,
    STATE(204), 1,
      sym_variable_ref,
    ACTIONS(539), 2,
      anon_sym_in,
      sym_identifier,
    STATE(175), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__meta_inline_args_repeat1,
    ACTIONS(537), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7628] = 10,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(333), 1,
      anon_sym_DOLLAR,
    ACTIONS(463), 1,
      anon_sym_in,
    ACTIONS(547), 1,
      sym_identifier,
    STATE(175), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(190), 1,
      sym__meta_inline_arg,
    STATE(204), 1,
      sym_variable_ref,
    STATE(176), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(545), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7664] = 10,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(454), 1,
      anon_sym_DOLLAR,
    ACTIONS(547), 1,
      sym_property_name,
    STATE(178), 1,
      aux_sym__meta_inline_args_repeat1,
    STATE(189), 1,
      sym__meta_inline_arg,
    STATE(193), 1,
      sym_variable_ref,
    ACTIONS(444), 2,
      anon_sym_in,
      sym_identifier,
    STATE(177), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(545), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [7700] = 9,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(552), 1,
      sym_property_name,
    ACTIONS(554), 1,
      anon_sym_DOLLAR,
    STATE(189), 1,
      sym__meta_inline_arg,
    STATE(193), 1,
      sym_variable_ref,
    ACTIONS(549), 2,
      anon_sym_in,
      sym_identifier,
    STATE(178), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__meta_inline_args_repeat1,
    ACTIONS(537), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [7734] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(67), 1,
      sym_property_name,
    ACTIONS(557), 1,
      anon_sym_COMMA,
    STATE(153), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(509), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(179), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(41), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [7766] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(180), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(442), 3,
      anon_sym_in,
      anon_sym_as,
      sym_identifier,
    ACTIONS(437), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_COMMA,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_selector,
  [7792] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(515), 1,
      sym_property_name,
    ACTIONS(559), 1,
      anon_sym_COMMA,
    STATE(179), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(509), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(181), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(511), 6,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
  [7824] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    ACTIONS(563), 1,
      anon_sym_SEMI,
    STATE(200), 1,
      sym_block,
    STATE(182), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(561), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7853] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(183), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(565), 10,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7876] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(569), 1,
      anon_sym_LPAREN,
    STATE(184), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(567), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7901] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    ACTIONS(573), 1,
      anon_sym_SEMI,
    STATE(198), 1,
      sym_block,
    STATE(185), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(571), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7930] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(186), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(575), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7952] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(187), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(577), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7974] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(188), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(579), 9,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [7996] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(189), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(581), 3,
      anon_sym_in,
      sym_identifier,
      sym_property_name,
    ACTIONS(583), 5,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_DOLLAR,
  [8019] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(581), 2,
      anon_sym_in,
      sym_identifier,
    STATE(190), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(583), 6,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_DOLLAR,
      sym_selector,
  [8042] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(191), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(35), 3,
      anon_sym_in,
      sym_identifier,
      sym_property_name,
    ACTIONS(37), 5,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_DOLLAR,
  [8065] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(563), 1,
      anon_sym_SEMI,
    STATE(192), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(561), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8088] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(193), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(585), 3,
      anon_sym_in,
      sym_identifier,
      sym_property_name,
    ACTIONS(587), 5,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      anon_sym_DOLLAR,
  [8111] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(591), 1,
      anon_sym_SEMI,
    STATE(194), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(589), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8134] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(195), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(565), 8,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_LPAREN,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8155] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(196), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(593), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8176] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(147), 1,
      anon_sym_LBRACE,
    ACTIONS(595), 1,
      anon_sym_SEMI,
    STATE(224), 1,
      sym_block,
    STATE(197), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(571), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8203] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(599), 1,
      anon_sym_SEMI,
    STATE(198), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(597), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8226] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(573), 1,
      anon_sym_SEMI,
    STATE(199), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(571), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8249] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(603), 1,
      anon_sym_SEMI,
    STATE(200), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(601), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8272] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(605), 1,
      anon_sym_LPAREN,
    STATE(201), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(567), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8295] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(147), 1,
      anon_sym_LBRACE,
    ACTIONS(607), 1,
      anon_sym_SEMI,
    STATE(225), 1,
      sym_block,
    STATE(202), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(561), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8322] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(203), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(609), 8,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8343] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(585), 2,
      anon_sym_in,
      sym_identifier,
    STATE(204), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(587), 6,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_DOLLAR,
      sym_selector,
  [8366] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(205), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(579), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8386] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(206), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(571), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8406] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(207), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(577), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8426] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    STATE(246), 1,
      sym_meta_block,
    STATE(208), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(611), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8450] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    STATE(252), 1,
      sym_meta_block,
    STATE(209), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(613), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8474] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    STATE(245), 1,
      sym_meta_block,
    STATE(210), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(615), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8498] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(211), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(617), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8518] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(212), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(619), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8538] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(467), 1,
      anon_sym_LBRACE,
    STATE(236), 1,
      sym_meta_block,
    STATE(213), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(621), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8562] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(214), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(597), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8582] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(215), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(561), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8602] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(216), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(601), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8622] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(217), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(575), 7,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8642] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(218), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(623), 7,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8662] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(595), 1,
      anon_sym_SEMI,
    STATE(219), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(571), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8683] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(220), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(625), 6,
      anon_sym_AT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8702] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    STATE(270), 1,
      sym_meta_block,
    STATE(221), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(613), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [8725] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(627), 1,
      sym_identifier,
    ACTIONS(629), 1,
      anon_sym_emit,
    ACTIONS(631), 1,
      anon_sym_macro,
    ACTIONS(633), 1,
      anon_sym_primitive,
    ACTIONS(635), 2,
      anon_sym_capture_type,
      anon_sym_captureType,
    STATE(222), 2,
      sym_line_comment,
      sym_block_comment,
  [8752] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    STATE(272), 1,
      sym_meta_block,
    STATE(223), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(621), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [8775] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(637), 1,
      anon_sym_SEMI,
    STATE(224), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(597), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8796] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(639), 1,
      anon_sym_SEMI,
    STATE(225), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(601), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8817] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(226), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(641), 6,
      anon_sym_AT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8836] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    STATE(271), 1,
      sym_meta_block,
    STATE(227), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(615), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [8859] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(228), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(643), 6,
      anon_sym_AT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8878] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(607), 1,
      anon_sym_SEMI,
    STATE(229), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(561), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8899] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(230), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(645), 6,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8918] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(231), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(647), 6,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      sym_identifier,
      sym_selector,
  [8937] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(232), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(649), 6,
      anon_sym_AT,
      anon_sym_RBRACE,
      anon_sym_AMP,
      sym_property_name,
      anon_sym_DOLLAR,
      sym_selector,
  [8956] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(233), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(609), 6,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [8975] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(450), 1,
      anon_sym_LBRACE,
    STATE(274), 1,
      sym_meta_block,
    STATE(234), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(611), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [8998] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(235), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(593), 6,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_SEMI,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9017] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(236), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(651), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9035] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(237), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(561), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9053] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(238), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(645), 5,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      sym_property_name,
  [9071] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(239), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(653), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9089] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(240), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(617), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9107] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(241), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(655), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9125] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(41), 1,
      anon_sym_RPAREN,
    ACTIONS(659), 1,
      anon_sym_COMMA,
    STATE(261), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(657), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(242), 2,
      sym_line_comment,
      sym_block_comment,
  [9149] = 8,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    ACTIONS(661), 1,
      anon_sym_COMMA,
    ACTIONS(663), 1,
      anon_sym_LBRACE,
    STATE(275), 1,
      aux_sym_selector_list_repeat1,
    STATE(283), 1,
      aux_sym_selector_list_repeat2,
    STATE(243), 2,
      sym_line_comment,
      sym_block_comment,
  [9175] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(244), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(665), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9193] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(245), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(667), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9211] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(246), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(669), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9229] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(247), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(611), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9247] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(248), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(619), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9265] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(249), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(671), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9283] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(673), 1,
      sym_identifier,
    ACTIONS(675), 1,
      anon_sym_macro,
    ACTIONS(677), 1,
      anon_sym_primitive,
    ACTIONS(679), 2,
      anon_sym_capture_type,
      anon_sym_captureType,
    STATE(250), 2,
      sym_line_comment,
      sym_block_comment,
  [9307] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(251), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(647), 5,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_LBRACE,
      anon_sym_RBRACE,
      sym_property_name,
  [9325] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(252), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(681), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9343] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(253), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(613), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9361] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(254), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(683), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9379] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(255), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(615), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9397] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(256), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(685), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9415] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(257), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(621), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9433] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(258), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(687), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9451] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(259), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(571), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9469] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(260), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(601), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9487] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(437), 1,
      anon_sym_RPAREN,
    ACTIONS(689), 3,
      anon_sym_in,
      anon_sym_COMMA,
      anon_sym_as,
    STATE(261), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym__args_inner_repeat1,
  [9507] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(262), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(597), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9525] = 7,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(511), 1,
      anon_sym_RPAREN,
    ACTIONS(692), 1,
      anon_sym_COMMA,
    STATE(242), 1,
      aux_sym__args_inner_repeat1,
    ACTIONS(657), 2,
      anon_sym_in,
      anon_sym_as,
    STATE(263), 2,
      sym_line_comment,
      sym_block_comment,
  [9549] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(694), 1,
      anon_sym_EQ,
    STATE(264), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(89), 4,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
  [9569] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(265), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(696), 5,
      ts_builtin_sym_end,
      anon_sym_AT,
      anon_sym_PERCENT,
      sym_identifier,
      sym_selector,
  [9587] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(266), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(698), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9604] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(267), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(683), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9621] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    STATE(275), 1,
      aux_sym_selector_list_repeat1,
    ACTIONS(700), 2,
      anon_sym_COMMA,
      anon_sym_LBRACE,
    STATE(268), 2,
      sym_line_comment,
      sym_block_comment,
  [9642] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(269), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(437), 4,
      anon_sym_in,
      anon_sym_RPAREN,
      anon_sym_COMMA,
      anon_sym_as,
  [9659] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(270), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(681), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9676] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(271), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(667), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9693] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(272), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(651), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9710] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(273), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(611), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9727] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(274), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(669), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9744] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(704), 1,
      sym_selector,
    ACTIONS(702), 2,
      anon_sym_COMMA,
      anon_sym_LBRACE,
    STATE(275), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym_selector_list_repeat1,
  [9763] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(276), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(671), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9780] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(277), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(707), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9797] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(278), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(613), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9814] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(279), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(615), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9831] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(280), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(621), 4,
      anon_sym_AT,
      anon_sym_PERCENT,
      anon_sym_RBRACE,
      sym_property_name,
  [9848] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(700), 1,
      anon_sym_LBRACE,
    ACTIONS(709), 1,
      anon_sym_COMMA,
    STATE(281), 3,
      sym_line_comment,
      sym_block_comment,
      aux_sym_selector_list_repeat2,
  [9866] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    STATE(282), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(712), 3,
      anon_sym_COMMA,
      anon_sym_LBRACE,
      sym_selector,
  [9882] = 6,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(661), 1,
      anon_sym_COMMA,
    ACTIONS(714), 1,
      anon_sym_LBRACE,
    STATE(281), 1,
      aux_sym_selector_list_repeat2,
    STATE(283), 2,
      sym_line_comment,
      sym_block_comment,
  [9902] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(716), 1,
      anon_sym_RBRACE,
    ACTIONS(718), 1,
      sym_emit_content,
    STATE(284), 2,
      sym_line_comment,
      sym_block_comment,
  [9919] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(147), 1,
      anon_sym_LBRACE,
    STATE(239), 1,
      sym_block,
    STATE(285), 2,
      sym_line_comment,
      sym_block_comment,
  [9936] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(125), 1,
      anon_sym_LBRACE,
    STATE(232), 1,
      sym_block,
    STATE(286), 2,
      sym_line_comment,
      sym_block_comment,
  [9953] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(147), 1,
      anon_sym_LBRACE,
    STATE(241), 1,
      sym_block,
    STATE(287), 2,
      sym_line_comment,
      sym_block_comment,
  [9970] = 5,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(15), 1,
      sym_selector,
    STATE(268), 1,
      aux_sym_selector_list_repeat1,
    STATE(288), 2,
      sym_line_comment,
      sym_block_comment,
  [9987] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(720), 1,
      sym_identifier,
    STATE(289), 2,
      sym_line_comment,
      sym_block_comment,
  [10001] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(722), 1,
      sym_identifier,
    STATE(290), 2,
      sym_line_comment,
      sym_block_comment,
  [10015] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(724), 1,
      sym_identifier,
    STATE(291), 2,
      sym_line_comment,
      sym_block_comment,
  [10029] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(726), 1,
      anon_sym_COLON,
    STATE(292), 2,
      sym_line_comment,
      sym_block_comment,
  [10043] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(728), 1,
      sym_identifier,
    STATE(293), 2,
      sym_line_comment,
      sym_block_comment,
  [10057] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(730), 1,
      sym_identifier,
    STATE(294), 2,
      sym_line_comment,
      sym_block_comment,
  [10071] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(732), 1,
      sym_identifier,
    STATE(295), 2,
      sym_line_comment,
      sym_block_comment,
  [10085] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(734), 1,
      sym_identifier,
    STATE(296), 2,
      sym_line_comment,
      sym_block_comment,
  [10099] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(736), 1,
      sym_identifier,
    STATE(297), 2,
      sym_line_comment,
      sym_block_comment,
  [10113] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(738), 1,
      sym_identifier,
    STATE(298), 2,
      sym_line_comment,
      sym_block_comment,
  [10127] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(227), 1,
      anon_sym_RPAREN,
    STATE(299), 2,
      sym_line_comment,
      sym_block_comment,
  [10141] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(319), 1,
      anon_sym_RPAREN,
    STATE(300), 2,
      sym_line_comment,
      sym_block_comment,
  [10155] = 4,
    ACTIONS(740), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(742), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(744), 1,
      aux_sym_block_comment_token1,
    STATE(301), 2,
      sym_line_comment,
      sym_block_comment,
  [10169] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(746), 1,
      anon_sym_RPAREN,
    STATE(302), 2,
      sym_line_comment,
      sym_block_comment,
  [10183] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(748), 1,
      anon_sym_RPAREN,
    STATE(303), 2,
      sym_line_comment,
      sym_block_comment,
  [10197] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(750), 1,
      sym_identifier,
    STATE(304), 2,
      sym_line_comment,
      sym_block_comment,
  [10211] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(752), 1,
      sym_identifier,
    STATE(305), 2,
      sym_line_comment,
      sym_block_comment,
  [10225] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(754), 1,
      anon_sym_RPAREN,
    STATE(306), 2,
      sym_line_comment,
      sym_block_comment,
  [10239] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(756), 1,
      anon_sym_RPAREN,
    STATE(307), 2,
      sym_line_comment,
      sym_block_comment,
  [10253] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(758), 1,
      sym_identifier,
    STATE(308), 2,
      sym_line_comment,
      sym_block_comment,
  [10267] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(760), 1,
      sym_identifier,
    STATE(309), 2,
      sym_line_comment,
      sym_block_comment,
  [10281] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(762), 1,
      sym_identifier,
    STATE(310), 2,
      sym_line_comment,
      sym_block_comment,
  [10295] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(764), 1,
      sym_identifier,
    STATE(311), 2,
      sym_line_comment,
      sym_block_comment,
  [10309] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(766), 1,
      anon_sym_SEMI,
    STATE(312), 2,
      sym_line_comment,
      sym_block_comment,
  [10323] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(768), 1,
      sym_identifier,
    STATE(313), 2,
      sym_line_comment,
      sym_block_comment,
  [10337] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(770), 1,
      sym_identifier,
    STATE(314), 2,
      sym_line_comment,
      sym_block_comment,
  [10351] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(772), 1,
      anon_sym_SLASH,
    STATE(315), 2,
      sym_line_comment,
      sym_block_comment,
  [10365] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(774), 1,
      anon_sym_RPAREN,
    STATE(316), 2,
      sym_line_comment,
      sym_block_comment,
  [10379] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(776), 1,
      sym_identifier,
    STATE(317), 2,
      sym_line_comment,
      sym_block_comment,
  [10393] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(778), 1,
      anon_sym_RPAREN,
    STATE(318), 2,
      sym_line_comment,
      sym_block_comment,
  [10407] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(780), 1,
      sym_identifier,
    STATE(319), 2,
      sym_line_comment,
      sym_block_comment,
  [10421] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(782), 1,
      sym_identifier,
    STATE(320), 2,
      sym_line_comment,
      sym_block_comment,
  [10435] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(784), 1,
      sym_identifier,
    STATE(321), 2,
      sym_line_comment,
      sym_block_comment,
  [10449] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(786), 1,
      anon_sym_RPAREN,
    STATE(322), 2,
      sym_line_comment,
      sym_block_comment,
  [10463] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(788), 1,
      anon_sym_RPAREN,
    STATE(323), 2,
      sym_line_comment,
      sym_block_comment,
  [10477] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(790), 1,
      sym_identifier,
    STATE(324), 2,
      sym_line_comment,
      sym_block_comment,
  [10491] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(792), 1,
      sym_identifier,
    STATE(325), 2,
      sym_line_comment,
      sym_block_comment,
  [10505] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(794), 1,
      sym_identifier,
    STATE(326), 2,
      sym_line_comment,
      sym_block_comment,
  [10519] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(796), 1,
      anon_sym_LBRACE,
    STATE(327), 2,
      sym_line_comment,
      sym_block_comment,
  [10533] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(798), 1,
      anon_sym_RPAREN,
    STATE(328), 2,
      sym_line_comment,
      sym_block_comment,
  [10547] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(800), 1,
      sym_identifier,
    STATE(329), 2,
      sym_line_comment,
      sym_block_comment,
  [10561] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(802), 1,
      sym_identifier,
    STATE(330), 2,
      sym_line_comment,
      sym_block_comment,
  [10575] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(804), 1,
      sym_identifier,
    STATE(331), 2,
      sym_line_comment,
      sym_block_comment,
  [10589] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(806), 1,
      sym_identifier,
    STATE(332), 2,
      sym_line_comment,
      sym_block_comment,
  [10603] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(808), 1,
      sym_identifier,
    STATE(333), 2,
      sym_line_comment,
      sym_block_comment,
  [10617] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(810), 1,
      sym_identifier,
    STATE(334), 2,
      sym_line_comment,
      sym_block_comment,
  [10631] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(812), 1,
      sym_identifier,
    STATE(335), 2,
      sym_line_comment,
      sym_block_comment,
  [10645] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(814), 1,
      sym_identifier,
    STATE(336), 2,
      sym_line_comment,
      sym_block_comment,
  [10659] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(816), 1,
      anon_sym_COLON,
    STATE(337), 2,
      sym_line_comment,
      sym_block_comment,
  [10673] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(818), 1,
      anon_sym_RBRACE,
    STATE(338), 2,
      sym_line_comment,
      sym_block_comment,
  [10687] = 4,
    ACTIONS(740), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(742), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(820), 1,
      aux_sym_line_comment_token1,
    STATE(339), 2,
      sym_line_comment,
      sym_block_comment,
  [10701] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(822), 1,
      ts_builtin_sym_end,
    STATE(340), 2,
      sym_line_comment,
      sym_block_comment,
  [10715] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(824), 1,
      sym_identifier,
    STATE(341), 2,
      sym_line_comment,
      sym_block_comment,
  [10729] = 4,
    ACTIONS(3), 1,
      anon_sym_SLASH_SLASH,
    ACTIONS(5), 1,
      anon_sym_SLASH_STAR,
    ACTIONS(826), 1,
      anon_sym_COLON,
    STATE(342), 2,
      sym_line_comment,
      sym_block_comment,
  [10743] = 1,
    ACTIONS(828), 1,
      ts_builtin_sym_end,
  [10747] = 1,
    ACTIONS(830), 1,
      ts_builtin_sym_end,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(5)] = 0,
  [SMALL_STATE(6)] = 47,
  [SMALL_STATE(7)] = 90,
  [SMALL_STATE(8)] = 135,
  [SMALL_STATE(9)] = 195,
  [SMALL_STATE(10)] = 237,
  [SMALL_STATE(11)] = 283,
  [SMALL_STATE(12)] = 343,
  [SMALL_STATE(13)] = 385,
  [SMALL_STATE(14)] = 427,
  [SMALL_STATE(15)] = 469,
  [SMALL_STATE(16)] = 515,
  [SMALL_STATE(17)] = 557,
  [SMALL_STATE(18)] = 621,
  [SMALL_STATE(19)] = 665,
  [SMALL_STATE(20)] = 729,
  [SMALL_STATE(21)] = 771,
  [SMALL_STATE(22)] = 831,
  [SMALL_STATE(23)] = 891,
  [SMALL_STATE(24)] = 933,
  [SMALL_STATE(25)] = 979,
  [SMALL_STATE(26)] = 1021,
  [SMALL_STATE(27)] = 1092,
  [SMALL_STATE(28)] = 1163,
  [SMALL_STATE(29)] = 1232,
  [SMALL_STATE(30)] = 1277,
  [SMALL_STATE(31)] = 1348,
  [SMALL_STATE(32)] = 1419,
  [SMALL_STATE(33)] = 1460,
  [SMALL_STATE(34)] = 1503,
  [SMALL_STATE(35)] = 1572,
  [SMALL_STATE(36)] = 1612,
  [SMALL_STATE(37)] = 1652,
  [SMALL_STATE(38)] = 1694,
  [SMALL_STATE(39)] = 1750,
  [SMALL_STATE(40)] = 1790,
  [SMALL_STATE(41)] = 1834,
  [SMALL_STATE(42)] = 1874,
  [SMALL_STATE(43)] = 1914,
  [SMALL_STATE(44)] = 1954,
  [SMALL_STATE(45)] = 1994,
  [SMALL_STATE(46)] = 2034,
  [SMALL_STATE(47)] = 2078,
  [SMALL_STATE(48)] = 2122,
  [SMALL_STATE(49)] = 2181,
  [SMALL_STATE(50)] = 2240,
  [SMALL_STATE(51)] = 2297,
  [SMALL_STATE(52)] = 2350,
  [SMALL_STATE(53)] = 2412,
  [SMALL_STATE(54)] = 2466,
  [SMALL_STATE(55)] = 2522,
  [SMALL_STATE(56)] = 2584,
  [SMALL_STATE(57)] = 2642,
  [SMALL_STATE(58)] = 2704,
  [SMALL_STATE(59)] = 2766,
  [SMALL_STATE(60)] = 2828,
  [SMALL_STATE(61)] = 2890,
  [SMALL_STATE(62)] = 2928,
  [SMALL_STATE(63)] = 2990,
  [SMALL_STATE(64)] = 3052,
  [SMALL_STATE(65)] = 3108,
  [SMALL_STATE(66)] = 3170,
  [SMALL_STATE(67)] = 3232,
  [SMALL_STATE(68)] = 3294,
  [SMALL_STATE(69)] = 3353,
  [SMALL_STATE(70)] = 3412,
  [SMALL_STATE(71)] = 3471,
  [SMALL_STATE(72)] = 3530,
  [SMALL_STATE(73)] = 3589,
  [SMALL_STATE(74)] = 3645,
  [SMALL_STATE(75)] = 3701,
  [SMALL_STATE(76)] = 3757,
  [SMALL_STATE(77)] = 3794,
  [SMALL_STATE(78)] = 3845,
  [SMALL_STATE(79)] = 3896,
  [SMALL_STATE(80)] = 3947,
  [SMALL_STATE(81)] = 3982,
  [SMALL_STATE(82)] = 4033,
  [SMALL_STATE(83)] = 4068,
  [SMALL_STATE(84)] = 4103,
  [SMALL_STATE(85)] = 4138,
  [SMALL_STATE(86)] = 4189,
  [SMALL_STATE(87)] = 4228,
  [SMALL_STATE(88)] = 4279,
  [SMALL_STATE(89)] = 4330,
  [SMALL_STATE(90)] = 4381,
  [SMALL_STATE(91)] = 4432,
  [SMALL_STATE(92)] = 4483,
  [SMALL_STATE(93)] = 4534,
  [SMALL_STATE(94)] = 4569,
  [SMALL_STATE(95)] = 4620,
  [SMALL_STATE(96)] = 4671,
  [SMALL_STATE(97)] = 4706,
  [SMALL_STATE(98)] = 4740,
  [SMALL_STATE(99)] = 4774,
  [SMALL_STATE(100)] = 4808,
  [SMALL_STATE(101)] = 4844,
  [SMALL_STATE(102)] = 4896,
  [SMALL_STATE(103)] = 4930,
  [SMALL_STATE(104)] = 4982,
  [SMALL_STATE(105)] = 5016,
  [SMALL_STATE(106)] = 5050,
  [SMALL_STATE(107)] = 5084,
  [SMALL_STATE(108)] = 5118,
  [SMALL_STATE(109)] = 5152,
  [SMALL_STATE(110)] = 5186,
  [SMALL_STATE(111)] = 5220,
  [SMALL_STATE(112)] = 5253,
  [SMALL_STATE(113)] = 5288,
  [SMALL_STATE(114)] = 5321,
  [SMALL_STATE(115)] = 5354,
  [SMALL_STATE(116)] = 5387,
  [SMALL_STATE(117)] = 5420,
  [SMALL_STATE(118)] = 5455,
  [SMALL_STATE(119)] = 5490,
  [SMALL_STATE(120)] = 5523,
  [SMALL_STATE(121)] = 5558,
  [SMALL_STATE(122)] = 5593,
  [SMALL_STATE(123)] = 5625,
  [SMALL_STATE(124)] = 5677,
  [SMALL_STATE(125)] = 5729,
  [SMALL_STATE(126)] = 5761,
  [SMALL_STATE(127)] = 5793,
  [SMALL_STATE(128)] = 5825,
  [SMALL_STATE(129)] = 5875,
  [SMALL_STATE(130)] = 5907,
  [SMALL_STATE(131)] = 5939,
  [SMALL_STATE(132)] = 5973,
  [SMALL_STATE(133)] = 6005,
  [SMALL_STATE(134)] = 6037,
  [SMALL_STATE(135)] = 6069,
  [SMALL_STATE(136)] = 6115,
  [SMALL_STATE(137)] = 6147,
  [SMALL_STATE(138)] = 6199,
  [SMALL_STATE(139)] = 6247,
  [SMALL_STATE(140)] = 6279,
  [SMALL_STATE(141)] = 6313,
  [SMALL_STATE(142)] = 6365,
  [SMALL_STATE(143)] = 6397,
  [SMALL_STATE(144)] = 6428,
  [SMALL_STATE(145)] = 6471,
  [SMALL_STATE(146)] = 6514,
  [SMALL_STATE(147)] = 6545,
  [SMALL_STATE(148)] = 6588,
  [SMALL_STATE(149)] = 6619,
  [SMALL_STATE(150)] = 6650,
  [SMALL_STATE(151)] = 6682,
  [SMALL_STATE(152)] = 6714,
  [SMALL_STATE(153)] = 6746,
  [SMALL_STATE(154)] = 6779,
  [SMALL_STATE(155)] = 6826,
  [SMALL_STATE(156)] = 6873,
  [SMALL_STATE(157)] = 6920,
  [SMALL_STATE(158)] = 6967,
  [SMALL_STATE(159)] = 7014,
  [SMALL_STATE(160)] = 7061,
  [SMALL_STATE(161)] = 7092,
  [SMALL_STATE(162)] = 7139,
  [SMALL_STATE(163)] = 7186,
  [SMALL_STATE(164)] = 7224,
  [SMALL_STATE(165)] = 7252,
  [SMALL_STATE(166)] = 7290,
  [SMALL_STATE(167)] = 7326,
  [SMALL_STATE(168)] = 7360,
  [SMALL_STATE(169)] = 7398,
  [SMALL_STATE(170)] = 7436,
  [SMALL_STATE(171)] = 7470,
  [SMALL_STATE(172)] = 7503,
  [SMALL_STATE(173)] = 7534,
  [SMALL_STATE(174)] = 7563,
  [SMALL_STATE(175)] = 7596,
  [SMALL_STATE(176)] = 7628,
  [SMALL_STATE(177)] = 7664,
  [SMALL_STATE(178)] = 7700,
  [SMALL_STATE(179)] = 7734,
  [SMALL_STATE(180)] = 7766,
  [SMALL_STATE(181)] = 7792,
  [SMALL_STATE(182)] = 7824,
  [SMALL_STATE(183)] = 7853,
  [SMALL_STATE(184)] = 7876,
  [SMALL_STATE(185)] = 7901,
  [SMALL_STATE(186)] = 7930,
  [SMALL_STATE(187)] = 7952,
  [SMALL_STATE(188)] = 7974,
  [SMALL_STATE(189)] = 7996,
  [SMALL_STATE(190)] = 8019,
  [SMALL_STATE(191)] = 8042,
  [SMALL_STATE(192)] = 8065,
  [SMALL_STATE(193)] = 8088,
  [SMALL_STATE(194)] = 8111,
  [SMALL_STATE(195)] = 8134,
  [SMALL_STATE(196)] = 8155,
  [SMALL_STATE(197)] = 8176,
  [SMALL_STATE(198)] = 8203,
  [SMALL_STATE(199)] = 8226,
  [SMALL_STATE(200)] = 8249,
  [SMALL_STATE(201)] = 8272,
  [SMALL_STATE(202)] = 8295,
  [SMALL_STATE(203)] = 8322,
  [SMALL_STATE(204)] = 8343,
  [SMALL_STATE(205)] = 8366,
  [SMALL_STATE(206)] = 8386,
  [SMALL_STATE(207)] = 8406,
  [SMALL_STATE(208)] = 8426,
  [SMALL_STATE(209)] = 8450,
  [SMALL_STATE(210)] = 8474,
  [SMALL_STATE(211)] = 8498,
  [SMALL_STATE(212)] = 8518,
  [SMALL_STATE(213)] = 8538,
  [SMALL_STATE(214)] = 8562,
  [SMALL_STATE(215)] = 8582,
  [SMALL_STATE(216)] = 8602,
  [SMALL_STATE(217)] = 8622,
  [SMALL_STATE(218)] = 8642,
  [SMALL_STATE(219)] = 8662,
  [SMALL_STATE(220)] = 8683,
  [SMALL_STATE(221)] = 8702,
  [SMALL_STATE(222)] = 8725,
  [SMALL_STATE(223)] = 8752,
  [SMALL_STATE(224)] = 8775,
  [SMALL_STATE(225)] = 8796,
  [SMALL_STATE(226)] = 8817,
  [SMALL_STATE(227)] = 8836,
  [SMALL_STATE(228)] = 8859,
  [SMALL_STATE(229)] = 8878,
  [SMALL_STATE(230)] = 8899,
  [SMALL_STATE(231)] = 8918,
  [SMALL_STATE(232)] = 8937,
  [SMALL_STATE(233)] = 8956,
  [SMALL_STATE(234)] = 8975,
  [SMALL_STATE(235)] = 8998,
  [SMALL_STATE(236)] = 9017,
  [SMALL_STATE(237)] = 9035,
  [SMALL_STATE(238)] = 9053,
  [SMALL_STATE(239)] = 9071,
  [SMALL_STATE(240)] = 9089,
  [SMALL_STATE(241)] = 9107,
  [SMALL_STATE(242)] = 9125,
  [SMALL_STATE(243)] = 9149,
  [SMALL_STATE(244)] = 9175,
  [SMALL_STATE(245)] = 9193,
  [SMALL_STATE(246)] = 9211,
  [SMALL_STATE(247)] = 9229,
  [SMALL_STATE(248)] = 9247,
  [SMALL_STATE(249)] = 9265,
  [SMALL_STATE(250)] = 9283,
  [SMALL_STATE(251)] = 9307,
  [SMALL_STATE(252)] = 9325,
  [SMALL_STATE(253)] = 9343,
  [SMALL_STATE(254)] = 9361,
  [SMALL_STATE(255)] = 9379,
  [SMALL_STATE(256)] = 9397,
  [SMALL_STATE(257)] = 9415,
  [SMALL_STATE(258)] = 9433,
  [SMALL_STATE(259)] = 9451,
  [SMALL_STATE(260)] = 9469,
  [SMALL_STATE(261)] = 9487,
  [SMALL_STATE(262)] = 9507,
  [SMALL_STATE(263)] = 9525,
  [SMALL_STATE(264)] = 9549,
  [SMALL_STATE(265)] = 9569,
  [SMALL_STATE(266)] = 9587,
  [SMALL_STATE(267)] = 9604,
  [SMALL_STATE(268)] = 9621,
  [SMALL_STATE(269)] = 9642,
  [SMALL_STATE(270)] = 9659,
  [SMALL_STATE(271)] = 9676,
  [SMALL_STATE(272)] = 9693,
  [SMALL_STATE(273)] = 9710,
  [SMALL_STATE(274)] = 9727,
  [SMALL_STATE(275)] = 9744,
  [SMALL_STATE(276)] = 9763,
  [SMALL_STATE(277)] = 9780,
  [SMALL_STATE(278)] = 9797,
  [SMALL_STATE(279)] = 9814,
  [SMALL_STATE(280)] = 9831,
  [SMALL_STATE(281)] = 9848,
  [SMALL_STATE(282)] = 9866,
  [SMALL_STATE(283)] = 9882,
  [SMALL_STATE(284)] = 9902,
  [SMALL_STATE(285)] = 9919,
  [SMALL_STATE(286)] = 9936,
  [SMALL_STATE(287)] = 9953,
  [SMALL_STATE(288)] = 9970,
  [SMALL_STATE(289)] = 9987,
  [SMALL_STATE(290)] = 10001,
  [SMALL_STATE(291)] = 10015,
  [SMALL_STATE(292)] = 10029,
  [SMALL_STATE(293)] = 10043,
  [SMALL_STATE(294)] = 10057,
  [SMALL_STATE(295)] = 10071,
  [SMALL_STATE(296)] = 10085,
  [SMALL_STATE(297)] = 10099,
  [SMALL_STATE(298)] = 10113,
  [SMALL_STATE(299)] = 10127,
  [SMALL_STATE(300)] = 10141,
  [SMALL_STATE(301)] = 10155,
  [SMALL_STATE(302)] = 10169,
  [SMALL_STATE(303)] = 10183,
  [SMALL_STATE(304)] = 10197,
  [SMALL_STATE(305)] = 10211,
  [SMALL_STATE(306)] = 10225,
  [SMALL_STATE(307)] = 10239,
  [SMALL_STATE(308)] = 10253,
  [SMALL_STATE(309)] = 10267,
  [SMALL_STATE(310)] = 10281,
  [SMALL_STATE(311)] = 10295,
  [SMALL_STATE(312)] = 10309,
  [SMALL_STATE(313)] = 10323,
  [SMALL_STATE(314)] = 10337,
  [SMALL_STATE(315)] = 10351,
  [SMALL_STATE(316)] = 10365,
  [SMALL_STATE(317)] = 10379,
  [SMALL_STATE(318)] = 10393,
  [SMALL_STATE(319)] = 10407,
  [SMALL_STATE(320)] = 10421,
  [SMALL_STATE(321)] = 10435,
  [SMALL_STATE(322)] = 10449,
  [SMALL_STATE(323)] = 10463,
  [SMALL_STATE(324)] = 10477,
  [SMALL_STATE(325)] = 10491,
  [SMALL_STATE(326)] = 10505,
  [SMALL_STATE(327)] = 10519,
  [SMALL_STATE(328)] = 10533,
  [SMALL_STATE(329)] = 10547,
  [SMALL_STATE(330)] = 10561,
  [SMALL_STATE(331)] = 10575,
  [SMALL_STATE(332)] = 10589,
  [SMALL_STATE(333)] = 10603,
  [SMALL_STATE(334)] = 10617,
  [SMALL_STATE(335)] = 10631,
  [SMALL_STATE(336)] = 10645,
  [SMALL_STATE(337)] = 10659,
  [SMALL_STATE(338)] = 10673,
  [SMALL_STATE(339)] = 10687,
  [SMALL_STATE(340)] = 10701,
  [SMALL_STATE(341)] = 10715,
  [SMALL_STATE(342)] = 10729,
  [SMALL_STATE(343)] = 10743,
  [SMALL_STATE(344)] = 10747,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT(339),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(301),
  [7] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(285),
  [11] = {.entry = {.count = 1, .reusable = true}}, SHIFT(3),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(222),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(282),
  [17] = {.entry = {.count = 1, .reusable = false}}, SHIFT(30),
  [19] = {.entry = {.count = 1, .reusable = false}}, SHIFT(99),
  [21] = {.entry = {.count = 1, .reusable = false}}, SHIFT(28),
  [23] = {.entry = {.count = 1, .reusable = false}}, SHIFT(113),
  [25] = {.entry = {.count = 1, .reusable = false}}, SHIFT(31),
  [27] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__value, 1, 0, 0),
  [29] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__value, 1, 0, 0),
  [31] = {.entry = {.count = 1, .reusable = true}}, SHIFT(58),
  [33] = {.entry = {.count = 1, .reusable = true}}, SHIFT(92),
  [35] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_variable_ref, 2, 0, 0),
  [37] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_ref, 2, 0, 0),
  [39] = {.entry = {.count = 1, .reusable = true}}, SHIFT(290),
  [41] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__args_inner, 2, 0, 0),
  [43] = {.entry = {.count = 1, .reusable = true}}, SHIFT(311),
  [45] = {.entry = {.count = 1, .reusable = true}}, SHIFT(320),
  [47] = {.entry = {.count = 1, .reusable = true}}, SHIFT(313),
  [49] = {.entry = {.count = 1, .reusable = true}}, SHIFT(42),
  [51] = {.entry = {.count = 1, .reusable = false}}, SHIFT(42),
  [53] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_preset_ref, 2, 0, 0),
  [55] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_preset_ref, 2, 0, 0),
  [57] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__arg, 1, 0, 0),
  [59] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__arg, 1, 0, 0),
  [61] = {.entry = {.count = 1, .reusable = true}}, SHIFT(95),
  [63] = {.entry = {.count = 1, .reusable = false}}, SHIFT(95),
  [65] = {.entry = {.count = 1, .reusable = true}}, SHIFT(5),
  [67] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__args_inner, 2, 0, 0),
  [69] = {.entry = {.count = 1, .reusable = true}}, SHIFT(334),
  [71] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [73] = {.entry = {.count = 1, .reusable = false}}, SHIFT(20),
  [75] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__expression, 1, 0, 0),
  [77] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__expression, 1, 0, 0),
  [79] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_function_call, 3, 0, 0),
  [81] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_function_call, 3, 0, 0),
  [83] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_paren_expr, 3, 0, 0),
  [85] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_paren_expr, 3, 0, 0),
  [87] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__arg, 3, 0, 0),
  [89] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__arg, 3, 0, 0),
  [91] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_function_call, 4, 0, 0),
  [93] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_function_call, 4, 0, 0),
  [95] = {.entry = {.count = 1, .reusable = true}}, SHIFT(332),
  [97] = {.entry = {.count = 1, .reusable = true}}, SHIFT(336),
  [99] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__args_inner, 3, 0, 0),
  [101] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__args_inner, 3, 0, 0),
  [103] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_element_ref, 2, 0, 0),
  [105] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_element_ref, 2, 0, 0),
  [107] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__arg, 5, 0, 0),
  [109] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__arg, 5, 0, 0),
  [111] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_binary_expr, 3, 0, 0),
  [113] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_binary_expr, 3, 0, 0),
  [115] = {.entry = {.count = 1, .reusable = false}}, SHIFT(93),
  [117] = {.entry = {.count = 1, .reusable = false}}, SHIFT(96),
  [119] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_directive, 2, 0, 0),
  [121] = {.entry = {.count = 1, .reusable = true}}, SHIFT(215),
  [123] = {.entry = {.count = 1, .reusable = true}}, SHIFT(62),
  [125] = {.entry = {.count = 1, .reusable = true}}, SHIFT(124),
  [127] = {.entry = {.count = 1, .reusable = true}}, SHIFT(319),
  [129] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_directive, 2, 0, 0),
  [131] = {.entry = {.count = 1, .reusable = true}}, SHIFT(325),
  [133] = {.entry = {.count = 1, .reusable = true}}, SHIFT(321),
  [135] = {.entry = {.count = 1, .reusable = true}}, SHIFT(96),
  [137] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_generic_directive, 2, 0, 0),
  [139] = {.entry = {.count = 1, .reusable = false}}, SHIFT(97),
  [141] = {.entry = {.count = 1, .reusable = false}}, SHIFT(107),
  [143] = {.entry = {.count = 1, .reusable = true}}, SHIFT(259),
  [145] = {.entry = {.count = 1, .reusable = true}}, SHIFT(67),
  [147] = {.entry = {.count = 1, .reusable = true}}, SHIFT(137),
  [149] = {.entry = {.count = 1, .reusable = true}}, SHIFT(298),
  [151] = {.entry = {.count = 1, .reusable = true}}, SHIFT(297),
  [153] = {.entry = {.count = 1, .reusable = true}}, SHIFT(293),
  [155] = {.entry = {.count = 1, .reusable = true}}, SHIFT(107),
  [157] = {.entry = {.count = 1, .reusable = true}}, SHIFT(63),
  [159] = {.entry = {.count = 1, .reusable = true}}, SHIFT(90),
  [161] = {.entry = {.count = 1, .reusable = true}}, SHIFT(206),
  [163] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_generic_directive, 2, 0, 0),
  [165] = {.entry = {.count = 1, .reusable = true}}, SHIFT(305),
  [167] = {.entry = {.count = 1, .reusable = true}}, SHIFT(237),
  [169] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(93),
  [172] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(96),
  [175] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0),
  [177] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(319),
  [180] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0),
  [182] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(325),
  [185] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(321),
  [188] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(96),
  [191] = {.entry = {.count = 1, .reusable = true}}, SHIFT(87),
  [193] = {.entry = {.count = 1, .reusable = false}}, SHIFT(87),
  [195] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__inline_directive_args, 1, 0, 0),
  [197] = {.entry = {.count = 1, .reusable = true}}, SHIFT(71),
  [199] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__inline_directive_args, 1, 0, 0),
  [201] = {.entry = {.count = 1, .reusable = true}}, SHIFT(69),
  [203] = {.entry = {.count = 1, .reusable = true}}, SHIFT(72),
  [205] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(97),
  [208] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(107),
  [211] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(298),
  [214] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(297),
  [217] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(293),
  [220] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 2, 0, 0), SHIFT_REPEAT(107),
  [223] = {.entry = {.count = 1, .reusable = true}}, SHIFT(86),
  [225] = {.entry = {.count = 1, .reusable = true}}, SHIFT(91),
  [227] = {.entry = {.count = 1, .reusable = true}}, SHIFT(207),
  [229] = {.entry = {.count = 1, .reusable = true}}, SHIFT(294),
  [231] = {.entry = {.count = 1, .reusable = true}}, SHIFT(289),
  [233] = {.entry = {.count = 1, .reusable = true}}, SHIFT(296),
  [235] = {.entry = {.count = 1, .reusable = true}}, SHIFT(104),
  [237] = {.entry = {.count = 1, .reusable = false}}, SHIFT(104),
  [239] = {.entry = {.count = 1, .reusable = true}}, SHIFT(121),
  [241] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__property_value, 1, 0, 0),
  [243] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__property_value, 1, 0, 0),
  [245] = {.entry = {.count = 1, .reusable = true}}, SHIFT(326),
  [247] = {.entry = {.count = 1, .reusable = true}}, SHIFT(134),
  [249] = {.entry = {.count = 1, .reusable = false}}, SHIFT(134),
  [251] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(121),
  [254] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0),
  [256] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(324),
  [259] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0),
  [261] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(330),
  [264] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(326),
  [267] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(134),
  [270] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(134),
  [273] = {.entry = {.count = 1, .reusable = true}}, SHIFT(230),
  [275] = {.entry = {.count = 1, .reusable = true}}, SHIFT(120),
  [277] = {.entry = {.count = 1, .reusable = true}}, SHIFT(329),
  [279] = {.entry = {.count = 1, .reusable = true}}, SHIFT(335),
  [281] = {.entry = {.count = 1, .reusable = true}}, SHIFT(331),
  [283] = {.entry = {.count = 1, .reusable = true}}, SHIFT(133),
  [285] = {.entry = {.count = 1, .reusable = false}}, SHIFT(133),
  [287] = {.entry = {.count = 1, .reusable = true}}, SHIFT(102),
  [289] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [291] = {.entry = {.count = 1, .reusable = true}}, SHIFT(139),
  [293] = {.entry = {.count = 1, .reusable = true}}, SHIFT(129),
  [295] = {.entry = {.count = 1, .reusable = true}}, SHIFT(186),
  [297] = {.entry = {.count = 1, .reusable = true}}, SHIFT(44),
  [299] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(120),
  [302] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(329),
  [305] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(335),
  [308] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(331),
  [311] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(133),
  [314] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__property_value_repeat1, 2, 0, 0), SHIFT_REPEAT(133),
  [317] = {.entry = {.count = 1, .reusable = true}}, SHIFT(238),
  [319] = {.entry = {.count = 1, .reusable = true}}, SHIFT(187),
  [321] = {.entry = {.count = 1, .reusable = true}}, SHIFT(217),
  [323] = {.entry = {.count = 1, .reusable = true}}, SHIFT(79),
  [325] = {.entry = {.count = 1, .reusable = true}}, SHIFT(29),
  [327] = {.entry = {.count = 1, .reusable = true}}, SHIFT(88),
  [329] = {.entry = {.count = 1, .reusable = true}}, SHIFT(57),
  [331] = {.entry = {.count = 1, .reusable = true}}, SHIFT(76),
  [333] = {.entry = {.count = 1, .reusable = true}}, SHIFT(295),
  [335] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__inline_directive_args_repeat1, 1, 0, 0),
  [337] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__inline_directive_args_repeat1, 1, 0, 0),
  [339] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [341] = {.entry = {.count = 1, .reusable = true}}, SHIFT(81),
  [343] = {.entry = {.count = 1, .reusable = true}}, SHIFT(37),
  [345] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__inline_arg, 1, 0, 0),
  [347] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__inline_arg, 1, 0, 0),
  [349] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__known_directive_name, 1, 0, 0),
  [351] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__known_directive_name, 1, 0, 0),
  [353] = {.entry = {.count = 1, .reusable = true}}, SHIFT(304),
  [355] = {.entry = {.count = 1, .reusable = true}}, SHIFT(324),
  [357] = {.entry = {.count = 1, .reusable = true}}, SHIFT(330),
  [359] = {.entry = {.count = 1, .reusable = true}}, SHIFT(77),
  [361] = {.entry = {.count = 1, .reusable = false}}, SHIFT(77),
  [363] = {.entry = {.count = 1, .reusable = true}}, SHIFT(59),
  [365] = {.entry = {.count = 1, .reusable = true}}, SHIFT(60),
  [367] = {.entry = {.count = 1, .reusable = true}}, SHIFT(4),
  [369] = {.entry = {.count = 1, .reusable = true}}, SHIFT(203),
  [371] = {.entry = {.count = 1, .reusable = true}}, SHIFT(342),
  [373] = {.entry = {.count = 1, .reusable = true}}, SHIFT(196),
  [375] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0), SHIFT_REPEAT(4),
  [378] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0),
  [380] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0), SHIFT_REPEAT(298),
  [383] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0), SHIFT_REPEAT(342),
  [386] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0), SHIFT_REPEAT(297),
  [389] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0), SHIFT_REPEAT(282),
  [392] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__property_value_repeat1, 1, 0, 0),
  [394] = {.entry = {.count = 1, .reusable = true}}, SHIFT(147),
  [396] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__property_value_repeat1, 1, 0, 0),
  [398] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [400] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(285),
  [403] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(3),
  [406] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(222),
  [409] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(282),
  [412] = {.entry = {.count = 1, .reusable = true}}, SHIFT(235),
  [414] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [416] = {.entry = {.count = 1, .reusable = true}}, SHIFT(145),
  [418] = {.entry = {.count = 1, .reusable = true}}, SHIFT(233),
  [420] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_transition_arrow, 3, 0, 0),
  [422] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_transition_arrow, 3, 0, 0),
  [424] = {.entry = {.count = 1, .reusable = true}}, SHIFT(45),
  [426] = {.entry = {.count = 1, .reusable = true}}, SHIFT(78),
  [428] = {.entry = {.count = 1, .reusable = false}}, SHIFT(78),
  [430] = {.entry = {.count = 1, .reusable = true}}, SHIFT(14),
  [432] = {.entry = {.count = 1, .reusable = true}}, SHIFT(119),
  [434] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0), SHIFT_REPEAT(75),
  [437] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0),
  [439] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0), SHIFT_REPEAT(75),
  [442] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0),
  [444] = {.entry = {.count = 1, .reusable = false}}, SHIFT(193),
  [446] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_primitive_def, 3, 0, 1),
  [448] = {.entry = {.count = 1, .reusable = true}}, SHIFT(65),
  [450] = {.entry = {.count = 1, .reusable = true}}, SHIFT(163),
  [452] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_primitive_def, 3, 0, 1),
  [454] = {.entry = {.count = 1, .reusable = true}}, SHIFT(333),
  [456] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_capture_type_def, 3, 0, 1),
  [458] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_capture_type_def, 3, 0, 1),
  [460] = {.entry = {.count = 2, .reusable = false}}, REDUCE(sym_primitive_def, 3, 0, 1), SHIFT(204),
  [463] = {.entry = {.count = 1, .reusable = false}}, SHIFT(204),
  [465] = {.entry = {.count = 1, .reusable = true}}, SHIFT(55),
  [467] = {.entry = {.count = 1, .reusable = true}}, SHIFT(168),
  [469] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_macro_def, 3, 0, 1),
  [471] = {.entry = {.count = 2, .reusable = false}}, REDUCE(sym_macro_def, 3, 0, 1), SHIFT(204),
  [474] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_macro_def, 3, 0, 1),
  [476] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_meta_directive, 2, 0, 0),
  [478] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_meta_directive, 2, 0, 0),
  [480] = {.entry = {.count = 1, .reusable = true}}, SHIFT(85),
  [482] = {.entry = {.count = 2, .reusable = false}}, REDUCE(sym_capture_type_def, 3, 0, 1), SHIFT(204),
  [485] = {.entry = {.count = 2, .reusable = false}}, REDUCE(sym_meta_directive, 2, 0, 0), SHIFT(204),
  [488] = {.entry = {.count = 1, .reusable = true}}, SHIFT(2),
  [490] = {.entry = {.count = 1, .reusable = true}}, SHIFT(250),
  [492] = {.entry = {.count = 1, .reusable = true}}, SHIFT(276),
  [494] = {.entry = {.count = 1, .reusable = true}}, SHIFT(337),
  [496] = {.entry = {.count = 1, .reusable = true}}, SHIFT(254),
  [498] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_meta_block_repeat1, 2, 0, 0), SHIFT_REPEAT(2),
  [501] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_meta_block_repeat1, 2, 0, 0), SHIFT_REPEAT(250),
  [504] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_meta_block_repeat1, 2, 0, 0),
  [506] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_meta_block_repeat1, 2, 0, 0), SHIFT_REPEAT(337),
  [509] = {.entry = {.count = 1, .reusable = false}}, SHIFT(75),
  [511] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__args_inner, 1, 0, 0),
  [513] = {.entry = {.count = 1, .reusable = true}}, SHIFT(11),
  [515] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__args_inner, 1, 0, 0),
  [517] = {.entry = {.count = 1, .reusable = true}}, SHIFT(249),
  [519] = {.entry = {.count = 1, .reusable = true}}, SHIFT(267),
  [521] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [523] = {.entry = {.count = 1, .reusable = false}}, SHIFT(73),
  [525] = {.entry = {.count = 1, .reusable = true}}, SHIFT(21),
  [527] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0), SHIFT_REPEAT(73),
  [530] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0), SHIFT_REPEAT(73),
  [533] = {.entry = {.count = 1, .reusable = true}}, SHIFT(89),
  [535] = {.entry = {.count = 1, .reusable = true}}, SHIFT(8),
  [537] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__meta_inline_args_repeat1, 2, 0, 0),
  [539] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__meta_inline_args_repeat1, 2, 0, 0), SHIFT_REPEAT(204),
  [542] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__meta_inline_args_repeat1, 2, 0, 0), SHIFT_REPEAT(295),
  [545] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__meta_inline_args, 1, 0, 0),
  [547] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__meta_inline_args, 1, 0, 0),
  [549] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym__meta_inline_args_repeat1, 2, 0, 0), SHIFT_REPEAT(193),
  [552] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__meta_inline_args_repeat1, 2, 0, 0),
  [554] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__meta_inline_args_repeat1, 2, 0, 0), SHIFT_REPEAT(333),
  [557] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [559] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [561] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_directive, 3, 0, 0),
  [563] = {.entry = {.count = 1, .reusable = true}}, SHIFT(216),
  [565] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__inline_directive_args, 3, 0, 0),
  [567] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__directive_args, 1, 0, 0),
  [569] = {.entry = {.count = 1, .reusable = true}}, SHIFT(66),
  [571] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_generic_directive, 3, 0, 0),
  [573] = {.entry = {.count = 1, .reusable = true}}, SHIFT(214),
  [575] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__directive_args, 2, 0, 0),
  [577] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__directive_args, 3, 0, 0),
  [579] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__directive_args, 4, 0, 0),
  [581] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym__meta_inline_args_repeat1, 1, 0, 0),
  [583] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym__meta_inline_args_repeat1, 1, 0, 0),
  [585] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__meta_inline_arg, 1, 0, 0),
  [587] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__meta_inline_arg, 1, 0, 0),
  [589] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_property_decl, 3, 0, 0),
  [591] = {.entry = {.count = 1, .reusable = true}}, SHIFT(218),
  [593] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 2, 0, 0),
  [595] = {.entry = {.count = 1, .reusable = true}}, SHIFT(262),
  [597] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_generic_directive, 4, 0, 0),
  [599] = {.entry = {.count = 1, .reusable = true}}, SHIFT(211),
  [601] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_directive, 4, 0, 0),
  [603] = {.entry = {.count = 1, .reusable = true}}, SHIFT(212),
  [605] = {.entry = {.count = 1, .reusable = true}}, SHIFT(52),
  [607] = {.entry = {.count = 1, .reusable = true}}, SHIFT(260),
  [609] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 3, 0, 0),
  [611] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_meta_directive, 3, 0, 0),
  [613] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_capture_type_def, 4, 0, 1),
  [615] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_primitive_def, 4, 0, 1),
  [617] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_generic_directive, 5, 0, 0),
  [619] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_directive, 5, 0, 0),
  [621] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_macro_def, 4, 0, 1),
  [623] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_property_decl, 4, 0, 0),
  [625] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_value_decl, 4, 0, 0),
  [627] = {.entry = {.count = 1, .reusable = false}}, SHIFT(162),
  [629] = {.entry = {.count = 1, .reusable = false}}, SHIFT(314),
  [631] = {.entry = {.count = 1, .reusable = false}}, SHIFT(310),
  [633] = {.entry = {.count = 1, .reusable = false}}, SHIFT(309),
  [635] = {.entry = {.count = 1, .reusable = false}}, SHIFT(308),
  [637] = {.entry = {.count = 1, .reusable = true}}, SHIFT(240),
  [639] = {.entry = {.count = 1, .reusable = true}}, SHIFT(248),
  [641] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 1, 0, 0),
  [643] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__block_item, 1, 0, 0),
  [645] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__meta_inline_args, 2, 0, 0),
  [647] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__meta_inline_args, 3, 0, 0),
  [649] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_nested_scope, 2, 0, 0),
  [651] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_macro_def, 5, 0, 1),
  [653] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope_block, 2, -10, 0),
  [655] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope_block, 2, 0, 0),
  [657] = {.entry = {.count = 1, .reusable = true}}, SHIFT(74),
  [659] = {.entry = {.count = 1, .reusable = true}}, SHIFT(70),
  [661] = {.entry = {.count = 1, .reusable = true}}, SHIFT(288),
  [663] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_selector_list, 1, 0, 0),
  [665] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_emit_directive, 5, 0, 0),
  [667] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_primitive_def, 5, 0, 1),
  [669] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_meta_directive, 4, 0, 0),
  [671] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_meta_block, 2, 0, 0),
  [673] = {.entry = {.count = 1, .reusable = false}}, SHIFT(159),
  [675] = {.entry = {.count = 1, .reusable = false}}, SHIFT(341),
  [677] = {.entry = {.count = 1, .reusable = false}}, SHIFT(317),
  [679] = {.entry = {.count = 1, .reusable = false}}, SHIFT(291),
  [681] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_capture_type_def, 5, 0, 1),
  [683] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_meta_block, 3, 0, 0),
  [685] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__item, 1, 0, 0),
  [687] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 1, 0, 0),
  [689] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym__args_inner_repeat1, 2, 0, 0), SHIFT_REPEAT(74),
  [692] = {.entry = {.count = 1, .reusable = true}}, SHIFT(68),
  [694] = {.entry = {.count = 1, .reusable = true}}, SHIFT(94),
  [696] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_emit_directive, 6, 0, 0),
  [698] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__meta_item, 1, 0, 0),
  [700] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_selector_list_repeat2, 2, 0, 0),
  [702] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_selector_list_repeat1, 2, 0, 0),
  [704] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_selector_list_repeat1, 2, 0, 0), SHIFT_REPEAT(282),
  [707] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_meta_block_repeat1, 1, 0, 0),
  [709] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_selector_list_repeat2, 2, 0, 0), SHIFT_REPEAT(288),
  [712] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_selector_list_repeat1, 1, 0, 0),
  [714] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_selector_list, 2, 0, 0),
  [716] = {.entry = {.count = 1, .reusable = true}}, SHIFT(244),
  [718] = {.entry = {.count = 1, .reusable = true}}, SHIFT(338),
  [720] = {.entry = {.count = 1, .reusable = true}}, SHIFT(106),
  [722] = {.entry = {.count = 1, .reusable = true}}, SHIFT(160),
  [724] = {.entry = {.count = 1, .reusable = true}}, SHIFT(155),
  [726] = {.entry = {.count = 1, .reusable = true}}, SHIFT(144),
  [728] = {.entry = {.count = 1, .reusable = true}}, SHIFT(108),
  [730] = {.entry = {.count = 1, .reusable = true}}, SHIFT(115),
  [732] = {.entry = {.count = 1, .reusable = true}}, SHIFT(61),
  [734] = {.entry = {.count = 1, .reusable = true}}, SHIFT(114),
  [736] = {.entry = {.count = 1, .reusable = true}}, SHIFT(109),
  [738] = {.entry = {.count = 1, .reusable = true}}, SHIFT(110),
  [740] = {.entry = {.count = 1, .reusable = false}}, SHIFT(339),
  [742] = {.entry = {.count = 1, .reusable = false}}, SHIFT(301),
  [744] = {.entry = {.count = 1, .reusable = false}}, SHIFT(315),
  [746] = {.entry = {.count = 1, .reusable = true}}, SHIFT(251),
  [748] = {.entry = {.count = 1, .reusable = true}}, SHIFT(231),
  [750] = {.entry = {.count = 1, .reusable = true}}, SHIFT(264),
  [752] = {.entry = {.count = 1, .reusable = true}}, SHIFT(173),
  [754] = {.entry = {.count = 1, .reusable = true}}, SHIFT(188),
  [756] = {.entry = {.count = 1, .reusable = true}}, SHIFT(41),
  [758] = {.entry = {.count = 1, .reusable = true}}, SHIFT(161),
  [760] = {.entry = {.count = 1, .reusable = true}}, SHIFT(156),
  [762] = {.entry = {.count = 1, .reusable = true}}, SHIFT(157),
  [764] = {.entry = {.count = 1, .reusable = true}}, SHIFT(35),
  [766] = {.entry = {.count = 1, .reusable = true}}, SHIFT(220),
  [768] = {.entry = {.count = 1, .reusable = true}}, SHIFT(39),
  [770] = {.entry = {.count = 1, .reusable = true}}, SHIFT(327),
  [772] = {.entry = {.count = 1, .reusable = false}}, SHIFT(344),
  [774] = {.entry = {.count = 1, .reusable = true}}, SHIFT(105),
  [776] = {.entry = {.count = 1, .reusable = true}}, SHIFT(154),
  [778] = {.entry = {.count = 1, .reusable = true}}, SHIFT(132),
  [780] = {.entry = {.count = 1, .reusable = true}}, SHIFT(80),
  [782] = {.entry = {.count = 1, .reusable = true}}, SHIFT(32),
  [784] = {.entry = {.count = 1, .reusable = true}}, SHIFT(83),
  [786] = {.entry = {.count = 1, .reusable = true}}, SHIFT(205),
  [788] = {.entry = {.count = 1, .reusable = true}}, SHIFT(142),
  [790] = {.entry = {.count = 1, .reusable = true}}, SHIFT(126),
  [792] = {.entry = {.count = 1, .reusable = true}}, SHIFT(82),
  [794] = {.entry = {.count = 1, .reusable = true}}, SHIFT(130),
  [796] = {.entry = {.count = 1, .reusable = true}}, SHIFT(284),
  [798] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [800] = {.entry = {.count = 1, .reusable = true}}, SHIFT(136),
  [802] = {.entry = {.count = 1, .reusable = true}}, SHIFT(127),
  [804] = {.entry = {.count = 1, .reusable = true}}, SHIFT(122),
  [806] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [808] = {.entry = {.count = 1, .reusable = true}}, SHIFT(191),
  [810] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [812] = {.entry = {.count = 1, .reusable = true}}, SHIFT(125),
  [814] = {.entry = {.count = 1, .reusable = true}}, SHIFT(6),
  [816] = {.entry = {.count = 1, .reusable = true}}, SHIFT(103),
  [818] = {.entry = {.count = 1, .reusable = true}}, SHIFT(265),
  [820] = {.entry = {.count = 1, .reusable = false}}, SHIFT(343),
  [822] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [824] = {.entry = {.count = 1, .reusable = true}}, SHIFT(158),
  [826] = {.entry = {.count = 1, .reusable = true}}, SHIFT(101),
  [828] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_line_comment, 2, 0, 0),
  [830] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block_comment, 3, 0, 0),
};

enum ts_external_scanner_symbol_identifiers {
  ts_external_token_emit_content = 0,
};

static const TSSymbol ts_external_scanner_symbol_map[EXTERNAL_TOKEN_COUNT] = {
  [ts_external_token_emit_content] = sym_emit_content,
};

static const bool ts_external_scanner_states[2][EXTERNAL_TOKEN_COUNT] = {
  [1] = {
    [ts_external_token_emit_content] = true,
  },
};

#ifdef __cplusplus
extern "C" {
#endif
void *tree_sitter_spacetime_external_scanner_create(void);
void tree_sitter_spacetime_external_scanner_destroy(void *);
bool tree_sitter_spacetime_external_scanner_scan(void *, TSLexer *, const bool *);
unsigned tree_sitter_spacetime_external_scanner_serialize(void *, char *);
void tree_sitter_spacetime_external_scanner_deserialize(void *, const char *, unsigned);

#ifdef TREE_SITTER_HIDE_SYMBOLS
#define TS_PUBLIC
#elif defined(_WIN32)
#define TS_PUBLIC __declspec(dllexport)
#else
#define TS_PUBLIC __attribute__((visibility("default")))
#endif

TS_PUBLIC const TSLanguage *tree_sitter_spacetime(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .field_names = ts_field_names,
    .field_map_slices = ts_field_map_slices,
    .field_map_entries = ts_field_map_entries,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .keyword_lex_fn = ts_lex_keywords,
    .keyword_capture_token = sym_identifier,
    .external_scanner = {
      &ts_external_scanner_states[0][0],
      ts_external_scanner_symbol_map,
      tree_sitter_spacetime_external_scanner_create,
      tree_sitter_spacetime_external_scanner_destroy,
      tree_sitter_spacetime_external_scanner_scan,
      tree_sitter_spacetime_external_scanner_serialize,
      tree_sitter_spacetime_external_scanner_deserialize,
    },
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
