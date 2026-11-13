use da20_core::types::AfmLookupTag;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const LOCALE_KEY: &str = "da20_locale_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Locale {
    #[default]
    Es,
    En,
}

impl Locale {
    pub fn load() -> Self {
        LocalStorage::get(LOCALE_KEY).unwrap_or(Self::Es)
    }

    pub fn save(self) {
        let _ = LocalStorage::set(LOCALE_KEY, self);
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Es => Self::En,
            Self::En => Self::Es,
        }
    }

    pub fn html_lang(self) -> &'static str {
        match self {
            Self::Es => "es",
            Self::En => "en",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Es => "ES",
            Self::En => "EN",
        }
    }
}

pub struct MethodCopy {
    pub title: &'static str,
    pub afm: &'static str,
    pub summary: &'static str,
    pub interpolation: &'static str,
    pub limitations: &'static str,
}

#[allow(dead_code)]
pub struct Copy {
    pub disclaimer: &'static str,
    pub logo_suffix: &'static str,
    pub tab_takeoff: &'static str,
    pub tab_climb: &'static str,
    pub tab_cruise: &'static str,
    pub tab_endurance: &'static str,
    pub tab_landing: &'static str,
    pub tab_method: &'static str,
    pub lang_toggle: &'static str,
    pub theme_toggle: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub theme_night: &'static str,
    pub palette_toggle: &'static str,
    pub palette_okabe_ito: &'static str,
    pub palette_vfr_chart: &'static str,
    pub palette_mono_amber: &'static str,
    pub palette_cividis: &'static str,
    pub subtitle: &'static str,
    pub inputs_title: &'static str,
    pub results_title: &'static str,
    pub save_readout: &'static str,
    pub readouts_title: &'static str,
    pub readout_label_hint: &'static str,
    pub clear_readouts: &'static str,
    pub overlay_on_chart: &'static str,
    pub results_unavailable: &'static str,
    pub export_csv: &'static str,
    pub export_json: &'static str,
    /// Primary chart PA input (standard pressure); same string in ES/EN per placard.
    pub pressure_altitude_ft: &'static str,
    pub ground_section_title: &'static str,
    pub ground_elevation_ft: &'static str,
    pub ground_oat_c: &'static str,
    pub oat: &'static str,
    pub runway: &'static str,
    pub runway_heading_ambiguous: &'static str,
    pub runway_heading_invalid: &'static str,
    pub wind: &'static str,
    pub wind_head: &'static str,
    pub wind_tail: &'static str,
    pub wind_cross: &'static str,
    pub wind_strip_crosswind_note: &'static str,
    pub results_runway_required: &'static str,
    pub weight: &'static str,
    pub cruise_pa: &'static str,
    pub cruise_rpm: &'static str,
    pub cruise_mode_set_rpm: &'static str,
    pub cruise_mode_set_bhp: &'static str,
    pub cruise_pct_bhp: &'static str,
    pub cruise_bhp_not_achievable: &'static str,
    pub propeller_toggle_label: &'static str,
    pub propeller_sensenich: &'static str,
    pub propeller_mt175r: &'static str,
    pub propeller_no_change_note: &'static str,
    pub density_altitude_readout: &'static str,
    pub mt_tcca_badge: &'static str,
    pub cruise_subtitle_table3: &'static str,
    pub cruise_subtitle_mt: &'static str,
    pub mt_fig2_title: &'static str,
    pub mt_fig3_title: &'static str,
    pub mt_fig4_col_da: &'static str,
    pub mt_fig4_col_rpm: &'static str,
    pub landing_tbl_roll_ft: &'static str,
    pub landing_tbl_50ft_ft: &'static str,
    pub mt_fig4_da_weight_lbl: &'static str,
    pub obstacle_m: &'static str,
    pub climb_figure: &'static str,
    pub climb_fig_55: &'static str,
    pub climb_fig_56: &'static str,
    pub climb_fig_58: &'static str,
    pub climb_flaps_to: &'static str,
    pub climb_flaps_up: &'static str,
    pub climb_flaps_ldg: &'static str,
    pub climb_flaps_balked_unavailable: &'static str,
    pub wheel_fairings_installed_yes: &'static str,
    pub wheel_fairings_installed_no: &'static str,
    /// Shared climb/cruise toggle legend (short placard label).
    pub wheel_fairings_toggle_label: &'static str,
    pub wheel_fairings_label_short: &'static str,
    pub climb_roc_fairing_penalty_suffix: &'static str,
    pub climb_roc_adjusted_label: &'static str,
    pub cruise_tas_fairing_penalty_suffix: &'static str,
    pub cruise_tas_adjusted_label: &'static str,
    pub endurance_range_adjusted_label: &'static str,
    pub afm_wheel_fairings_note_climb: &'static str,
    pub afm_wheel_fairings_note_cruise: &'static str,
    pub fuel_usable: &'static str,
    pub fuel_reserve: &'static str,
    pub derived_not_afm: &'static str,
    pub landing_no_corr: &'static str,
    pub landing_ground_idle_toggle: &'static str,
    pub landing_ground_idle_factor_prefix: &'static str,
    pub landing_ground_idle_off: &'static str,
    pub landing_ground_idle_on: &'static str,
    pub takeoff: &'static str,
    pub climb: &'static str,
    pub cruise: &'static str,
    pub landing: &'static str,
    pub wind_card: &'static str,
    pub unavailable: &'static str,
    pub derived_time: &'static str,
    pub plot_hint: &'static str,
    pub plot_readout_chart: &'static str,
    pub plot_readout_core: &'static str,
    pub pa_override_chip: &'static str,
    pub pa_override_reset: &'static str,
    pub axis_hw_kt: &'static str,
    pub axis_oat_c: &'static str,
    pub axis_base_roll_m: &'static str,
    pub axis_ea_ft: &'static str,
    pub axis_roc_fpm: &'static str,
    pub axis_pa_ft: &'static str,
    pub axis_dist_ft: &'static str,
    pub axis_angle_deg: &'static str,
    pub axis_weight_kg: &'static str,
    pub axis_tailwind_kt: &'static str,
    pub axis_obstacle_m: &'static str,
    pub axis_dist_m: &'static str,
    pub provenance_rev: &'static str,
    pub cruise_oat: &'static str,
    pub cruise_oat_estimated_label: &'static str,
    pub cruise_oat_use_estimate: &'static str,
    pub cruise_disa_readout: &'static str,
    pub cruise_isa: &'static str,
    pub cruise_disa_minus20: &'static str,
    pub cruise_disa_plus10: &'static str,
    /// AFM grid / interpolation tag (short, shown on results).
    pub afm_tag_table: &'static str,
    pub afm_tag_interpolated: &'static str,
    pub table_rows_used_title: &'static str,
    pub table_source_pa_lbl: &'static str,
    pub table_source_pa_weight_lbl: &'static str,
    pub table_source_rpm_lbl: &'static str,
    pub table_source_rpm_sample_lbl: &'static str,
    pub table_source_rpm_weight_lbl: &'static str,
    pub table_source_landing_roll_ft: &'static str,
    pub table_source_landing_over_50ft_ft: &'static str,
    pub cruise_tbl_group_disa_m20: &'static str,
    pub cruise_tbl_group_isa: &'static str,
    pub cruise_tbl_group_disa_p10: &'static str,
    pub cruise_tbl_pct_bhp: &'static str,
    pub cruise_tbl_ktas: &'static str,
    pub cruise_tbl_gph: &'static str,
    pub obstacle_preset_afm: &'static str,
    pub obstacle_preset_screen: &'static str,
    pub strip_fig54_base: &'static str,
    pub strip_fig54_wt: &'static str,
    pub strip_fig54_hw: &'static str,
    pub strip_fig54_tw: &'static str,
    pub strip_panel_oat_pa: &'static str,
    pub strip_panel_weight: &'static str,
    pub strip_panel_hw: &'static str,
    pub strip_panel_tw: &'static str,
    pub strip_panel_wind: &'static str,
    pub strip_panel_obstacle: &'static str,
    /// Fig 5.2 ROC panel bottom x-axis (Pablo).
    pub strip_climb_roc_x_axis: &'static str,
    /// ROC panel right edge: shared equivalent altitude axis (same scale as fan chart).
    pub strip_climb_roc_ea_y_axis: &'static str,
    pub strip_fig52_ea: &'static str,
    pub strip_fig55_roc: &'static str,
    pub strip_fig56_roc: &'static str,
    pub strip_fig58_roc: &'static str,
    pub takeoff_ground_roll: &'static str,
    pub takeoff_ground_roll_at_chart_zero: &'static str,
    pub takeoff_over_obstacle: &'static str,
    pub takeoff_signed_headwind: &'static str,
    pub takeoff_weight_kg: &'static str,
    pub obstacle_ft: &'static str,
    pub unit_kg: &'static str,
    pub unit_lb: &'static str,
    pub unit_m: &'static str,
    pub unit_ft: &'static str,
    pub takeoff_rounding_label: &'static str,
    pub takeoff_rounding_raw: &'static str,
    pub takeoff_rounding_ceil_10m: &'static str,
    pub takeoff_rounding_ceil_50ft: &'static str,
    pub takeoff_sweep_title: &'static str,
    pub takeoff_sweep_run: &'static str,
    pub takeoff_sweep_var_oat: &'static str,
    pub takeoff_sweep_var_hw: &'static str,
    pub takeoff_sweep_var_obs: &'static str,
    pub takeoff_sweep_start: &'static str,
    pub takeoff_sweep_end: &'static str,
    pub takeoff_sweep_step: &'static str,
    pub takeoff_sweep_col_gr: &'static str,
    pub takeoff_sweep_col_to: &'static str,
    pub takeoff_sweep_col_tag: &'static str,
    pub takeoff_sweep_oob_footnote: &'static str,
    pub manual_title: &'static str,
    pub manual_intro: &'static str,
    pub afm_link: &'static str,
    pub portal_link: &'static str,
    pub amm_link: &'static str,
    pub methods: &'static [MethodCopy],
}

pub fn copy(locale: Locale) -> &'static Copy {
    match locale {
        Locale::Es => &ES,
        Locale::En => &EN,
    }
}

pub fn format_mt_cruise_bhp_not_achievable(
    locale: Locale,
    pct_bhp: f64,
    rmin: f64,
    rmax: f64,
) -> String {
    match locale {
        Locale::Es => format!(
            "No alcanzable — %BHP {pct_bhp:.2}% fuera de [{rmin:.0}, {rmax:.0}] RPM a esta AD (Fig 4)."
        ),
        Locale::En => format!(
            "Not achievable — %BHP {pct_bhp:.2}% outside [{rmin:.0}, {rmax:.0}] RPM at this DA (Fig 4)."
        ),
    }
}

pub fn afm_lookup_tag_label(locale: Locale, tag: AfmLookupTag) -> &'static str {
    let c = copy(locale);
    match tag {
        AfmLookupTag::Table => c.afm_tag_table,
        AfmLookupTag::Interpolated => c.afm_tag_interpolated,
    }
}

const ES: Copy = Copy {
    disclaimer: "No es fuente oficial. No aprobado por Diamond Aircraft ni DGAC. Valores digitalizados y aproximados. El AFM aprobado rige toda la planificación de vuelo.",
    logo_suffix: "Performance",
    tab_takeoff: "Despegue",
    tab_climb: "Ascenso",
    tab_cruise: "Crucero y potencia",
    tab_endurance: "Autonomía y alcance",
    tab_landing: "Aterrizaje",
    tab_method: "Método",
    lang_toggle: "Idioma",
    theme_toggle: "Tema",
    theme_system: "Sistema",
    theme_light: "Claro",
    theme_dark: "Oscuro",
    theme_night: "Noche",
    palette_toggle: "Paleta",
    palette_okabe_ito: "Okabe-Ito",
    palette_vfr_chart: "Carta VFR",
    palette_mono_amber: "Mono alto contraste + ámbar",
    palette_cividis: "Derivado Cividis",
    subtitle: "AFM DA202-C1 Rev 29 (30 jun 2021) — Capítulo 5 digitalizado",
    inputs_title: "Entradas",
    results_title: "Resultados",
    save_readout: "Guardar lectura",
    readouts_title: "Lecturas guardadas",
    readout_label_hint: "Etiqueta (opcional)",
    clear_readouts: "Borrar todas",
    overlay_on_chart: "Superponer en gráfico",
    results_unavailable: "—",
    export_csv: "Exportar CSV",
    export_json: "Exportar JSON",
    pressure_altitude_ft: "Altitud de presión (ft)",
    ground_section_title: "Tierra / Aeródromo",
    ground_elevation_ft: "Elevación del aeródromo (ft)",
    ground_oat_c: "OAT en tierra (°C)",
    oat: "OAT (°C)",
    runway: "Orientación pista",
    runway_heading_ambiguous:
        "Use 3 dígitos magnéticos (001–360). «19» es ambiguo (¿019?).",
    runway_heading_invalid: "Rumbo inválido: ingrese 001–360 (3 dígitos).",
    wind: "Viento (° / kt)",
    wind_head: "Viento de frente",
    wind_tail: "Viento de cola",
    wind_cross: "Viento cruzado",
    wind_strip_crosswind_note:
        "Viento casi cruzado: en la franja de viento solo se ajusta la intensidad (kt), no la dirección.",
    results_runway_required: "Ingrese rumbo de pista válido (001–360) para ver resultados.",
    weight: "Peso (lb)",
    cruise_pa: "PA crucero (ft)",
    cruise_rpm: "RPM crucero",
    cruise_mode_set_rpm: "Fijar RPM",
    cruise_mode_set_bhp: "Fijar %BHP",
    cruise_pct_bhp: "% BHP crucero",
    cruise_bhp_not_achievable:
        "No alcanzable — %BHP sin raíz en [rmin(PA), 2800 RPM] (Tabla 3).",
    propeller_toggle_label: "Hélice / Propeller",
    propeller_sensenich: "Sensenich W69EK7-63",
    propeller_mt175r: "MT 175 R 150-2Ca",
    propeller_no_change_note:
        "Sin cambio según AFMS E-1487 / No change per AFMS E-1487",
    density_altitude_readout: "Alt. densidad (DA)",
    mt_tcca_badge: "Datos no aprobados por TCCA (AFMS E-1487)",
    cruise_subtitle_table3: "Tabla 3",
    cruise_subtitle_mt: "AFMS Fig 4 — MT 175 R",
    mt_fig2_title: "Fig 2 — RPM vs DA",
    mt_fig3_title: "Fig 3 — TAS vs DA",
    mt_fig4_col_da: "DA",
    mt_fig4_col_rpm: "RPM",
    landing_tbl_roll_ft: "Rodaje ft",
    landing_tbl_50ft_ft: "50 ft ft",
    mt_fig4_da_weight_lbl: "Peso DA",
    obstacle_m: "Altura obstáculo (m)",
    climb_figure: "Configuración de trepada (AFM)",
    climb_fig_55: "Fig 5.5 — flaps crucero",
    climb_fig_56: "Fig 5.6 — flaps despegue",
    climb_fig_58: "Fig 5.8 — aterrizaje abortado",
    climb_flaps_to: "Flaps T/O",
    climb_flaps_up: "Flaps UP/CRUISE",
    climb_flaps_ldg: "Flaps LDG - Balked Landing",
    climb_flaps_balked_unavailable:
        "Datos Fig 5.8 (aterrizaje abortado) no disponibles en el archivo de performance.",
    wheel_fairings_installed_yes: "Cubiertas de rueda instaladas",
    wheel_fairings_installed_no: "Sin cubiertas de rueda",
    wheel_fairings_toggle_label: "Carenados de rueda / Wheel fairings",
    wheel_fairings_label_short: "cubiertas",
    climb_roc_fairing_penalty_suffix: "-3% (sin carenados)",
    climb_roc_adjusted_label: "Ajustada",
    cruise_tas_fairing_penalty_suffix: "-5% (sin carenados)",
    cruise_tas_adjusted_label: "TAS ajustada",
    endurance_range_adjusted_label: "Rango ajustado",
    afm_wheel_fairings_note_climb:
        "CAUTION — IN AIRPLANE OPERATIONS WITHOUT THE OPTIONAL WHEEL FAIRINGS INSTALLED, THE CLIMB PERFORMANCE IS REDUCED BY APPROXIMATELY 3%.",
    afm_wheel_fairings_note_cruise:
        "CAUTION — IN AIRPLANE OPERATIONS WITHOUT THE OPTIONAL WHEEL FAIRINGS INSTALLED, THE MAXIMUM CRUISING SPEED IS REDUCED BY APPROXIMATELY 5%.",
    fuel_usable: "Combustible utilizable (US gal)",
    fuel_reserve: "Reserva (US gal)",
    derived_not_afm: "Derivado, no es carta AFM",
    landing_no_corr: "Sin corrección por viento, temperatura ni peso (Tabla 4).",
    landing_ground_idle_toggle: "Ralentí en tierra 1000 RPM",
    landing_ground_idle_factor_prefix:
        "NOTA §5.3.12 ralentí 1000 RPM: Tabla 4",
    landing_ground_idle_off: "No",
    landing_ground_idle_on: "Sí",
    takeoff: "Despegue (Fig 5.4)",
    climb: "Trepada (via Fig 5.2)",
    cruise: "Crucero",
    landing: "Aterrizaje (Tabla 4)",
    wind_card: "Viento (Fig 5.3)",
    unavailable: "No disponible",
    derived_time: "Tiempo estimado (sin carta AFM): ",
    plot_hint: "Arrastre el cursor sobre las curvas digitalizadas (no son copias del AFM). Compatible con puntero y táctil.",
    plot_readout_chart: "Lectura carta",
    plot_readout_core: "Núcleo",
    pa_override_chip: "PA carta",
    pa_override_reset: "Restablecer PA",
    axis_hw_kt: "HW kt",
    axis_oat_c: "OAT °C",
    axis_base_roll_m: "Rodaje base m",
    axis_ea_ft: "EA ft",
    axis_roc_fpm: "ROC fpm",
    axis_pa_ft: "PA ft",
    axis_dist_ft: "ft",
    axis_angle_deg: "Ángulo °",
    axis_weight_kg: "Peso (kg)",
    axis_tailwind_kt: "Viento de cola (kt)",
    axis_obstacle_m: "Obstáculo (m)",
    axis_dist_m: "m",
    provenance_rev: " rev ",
    cruise_oat: "OAT crucero (°C)",
    cruise_oat_estimated_label: "OAT estimada (gradiente ISA desde tierra) (°C)",
    cruise_oat_use_estimate: "Volver a OAT estimada desde tierra",
    cruise_disa_readout: "ΔISA (crucero)",
    cruise_isa: "ISA",
    cruise_disa_minus20: "dISA −20",
    cruise_disa_plus10: "dISA +10",
    afm_tag_table: "Tabla",
    afm_tag_interpolated: "Interpolado",
    table_rows_used_title: "Filas de la tabla",
    table_source_pa_lbl: "PA",
    table_source_pa_weight_lbl: "Peso PA",
    table_source_rpm_lbl: "RPM",
    table_source_rpm_sample_lbl: "RPM muestra",
    table_source_rpm_weight_lbl: "Peso RPM",
    table_source_landing_roll_ft: "Carrera (ft)",
    table_source_landing_over_50ft_ft: "Sobre 50 ft (ft)",
    cruise_tbl_group_disa_m20: "ΔISA −20",
    cruise_tbl_group_isa: "ISA",
    cruise_tbl_group_disa_p10: "ΔISA +10",
    cruise_tbl_pct_bhp: "% BHP",
    cruise_tbl_ktas: "TAS (kt)",
    cruise_tbl_gph: "GPH",
    obstacle_preset_afm: "5 m / 16 ft (AFM)",
    obstacle_preset_screen: "15 m / 50 ft",
    strip_fig54_base: "Fig 5.4 · base",
    strip_fig54_wt: "Peso",
    strip_fig54_hw: "Viento de frente",
    strip_fig54_tw: "Viento de cola",
    strip_panel_oat_pa: "OAT / PA",
    strip_panel_weight: "Peso",
    strip_panel_hw: "Viento de frente (kt)",
    strip_panel_tw: "Viento de cola (kt)",
    strip_panel_wind: "Viento de frente / cola (kt)",
    strip_panel_obstacle: "Obstáculo",
    strip_climb_roc_x_axis: "Razón de ascenso (fpm)",
    strip_climb_roc_ea_y_axis: "Altitud equivalente (ft)",
    strip_fig52_ea: "Fig 5.2 EA",
    strip_fig55_roc: "Fig 5.5 ROC",
    strip_fig56_roc: "Fig 5.6 ROC",
    strip_fig58_roc: "Fig 5.8 ROC",
    takeoff_ground_roll: "Rodaje",
    takeoff_ground_roll_at_chart_zero: "Rodaje (0 m)",
    takeoff_over_obstacle: "Sobre obstáculo de",
    takeoff_signed_headwind: "Viento en pista (kt, +frente / −cola)",
    takeoff_weight_kg: "Peso (kg)",
    obstacle_ft: "Altura obstáculo (ft)",
    unit_kg: "kg",
    unit_lb: "lb",
    unit_m: "m",
    unit_ft: "ft",
    takeoff_rounding_label: "Redondeo distancias",
    takeoff_rounding_raw: "Sin redondeo",
    takeoff_rounding_ceil_10m: "Al alza → 10 m",
    takeoff_rounding_ceil_50ft: "Conservador → 50 ft",
    takeoff_sweep_title: "Barrido de despegue",
    takeoff_sweep_run: "Calcular tabla",
    takeoff_sweep_var_oat: "OAT (°C)",
    takeoff_sweep_var_hw: "Viento en pista (kt)",
    takeoff_sweep_var_obs: "Obstáculo",
    takeoff_sweep_start: "Desde",
    takeoff_sweep_end: "Hasta",
    takeoff_sweep_step: "Paso",
    takeoff_sweep_col_gr: "Rodaje",
    takeoff_sweep_col_to: "Despegue",
    takeoff_sweep_col_tag: "Etiqueta",
    takeoff_sweep_oob_footnote: "— = combinación fuera de carta (solo interpolación, sin extrapolar).",
    manual_title: "Manual de métodos",
    manual_intro: "Cada cálculo, fuente AFM, interpolación y limitaciones.",
    afm_link: "AFM oficial Rev 29",
    portal_link: "Publicaciones técnicas Diamond",
    amm_link: "AMM DA201-C1 Rev 25",
    methods: &ES_METHODS,
};

const EN: Copy = Copy {
    disclaimer: "Not an official source. Not approved by Diamond Aircraft or DGAC. Digitized, approximate values. The approved AFM governs all flight planning.",
    logo_suffix: "Performance",
    tab_takeoff: "Takeoff",
    tab_climb: "Climb",
    tab_cruise: "Cruise & power",
    tab_endurance: "Endurance & range",
    tab_landing: "Landing",
    tab_method: "Method",
    lang_toggle: "Language",
    theme_toggle: "Theme",
    theme_system: "System",
    theme_light: "Light",
    theme_dark: "Dark",
    theme_night: "Night",
    palette_toggle: "Palette",
    palette_okabe_ito: "Okabe-Ito",
    palette_vfr_chart: "Aviation chart (VFR)",
    palette_mono_amber: "High-contrast mono + amber",
    palette_cividis: "Cividis-derived",
    subtitle: "AFM DA202-C1 Rev 29 (30 Jun 2021) — digitized Chapter 5",
    inputs_title: "Inputs",
    results_title: "Results",
    save_readout: "Save readout",
    readouts_title: "Saved readouts",
    readout_label_hint: "Label (optional)",
    clear_readouts: "Clear all",
    overlay_on_chart: "Overlay on chart",
    results_unavailable: "—",
    export_csv: "Export CSV",
    export_json: "Export JSON",
    pressure_altitude_ft: "Pressure altitude (ft)",
    ground_section_title: "Ground / Aerodrome",
    ground_elevation_ft: "Aerodrome elevation (ft)",
    ground_oat_c: "Ground OAT (°C)",
    oat: "OAT (°C)",
    runway: "Runway heading",
    runway_heading_ambiguous:
        "Use a 3-digit magnetic heading (001–360). “19” is ambiguous (019?).",
    runway_heading_invalid: "Invalid heading: enter 001–360 as three digits.",
    wind: "Wind (° / kt)",
    wind_head: "Headwind",
    wind_tail: "Tailwind",
    wind_cross: "Crosswind",
    wind_strip_crosswind_note:
        "Nearly pure crosswind: strip wind drag adjusts speed (kt) only, not direction.",
    results_runway_required: "Enter a valid runway heading (001–360) to show results.",
    weight: "Weight (lb)",
    cruise_pa: "Cruise PA (ft)",
    cruise_rpm: "Cruise RPM",
    cruise_mode_set_rpm: "Set RPM",
    cruise_mode_set_bhp: "Set %BHP",
    cruise_pct_bhp: "Cruise % BHP",
    cruise_bhp_not_achievable:
        "Not achievable — no RPM root on [rmin(PA), 2800] at this PA/ΔISA.",
    propeller_toggle_label: "Propeller / Hélice",
    propeller_sensenich: "Sensenich W69EK7-63",
    propeller_mt175r: "MT 175 R 150-2Ca",
    propeller_no_change_note:
        "No change per AFMS E-1487 / Sin cambio según AFMS E-1487",
    density_altitude_readout: "Density altitude (DA)",
    mt_tcca_badge: "Not TCCA-approved data (AFMS E-1487)",
    cruise_subtitle_table3: "Table 3",
    cruise_subtitle_mt: "AFMS Fig 4 — MT 175 R",
    mt_fig2_title: "Fig 2 — RPM vs DA",
    mt_fig3_title: "Fig 3 — TAS vs DA",
    mt_fig4_col_da: "DA",
    mt_fig4_col_rpm: "RPM",
    landing_tbl_roll_ft: "Roll ft",
    landing_tbl_50ft_ft: "50 ft ft",
    mt_fig4_da_weight_lbl: "DA weight",
    obstacle_m: "Obstacle height (m)",
    climb_figure: "Climb configuration (AFM)",
    climb_fig_55: "Fig 5.5 — cruise flaps",
    climb_fig_56: "Fig 5.6 — takeoff flaps",
    climb_fig_58: "Fig 5.8 — balked landing",
    climb_flaps_to: "Flaps T/O",
    climb_flaps_up: "Flaps UP/CRUISE",
    climb_flaps_ldg: "Flaps LDG - Balked Landing",
    climb_flaps_balked_unavailable:
        "Fig 5.8 (balked landing) performance data missing from performance file.",
    wheel_fairings_installed_yes: "Wheel fairings installed",
    wheel_fairings_installed_no: "Wheel fairings not installed",
    wheel_fairings_toggle_label: "Wheel fairings / Carenados de rueda",
    wheel_fairings_label_short: "fairings",
    climb_roc_fairing_penalty_suffix: "-3% (no fairings)",
    climb_roc_adjusted_label: "Adjusted",
    cruise_tas_fairing_penalty_suffix: "-5% (no fairings)",
    cruise_tas_adjusted_label: "Adjusted TAS",
    endurance_range_adjusted_label: "Adjusted range",
    afm_wheel_fairings_note_climb:
        "CAUTION — IN AIRPLANE OPERATIONS WITHOUT THE OPTIONAL WHEEL FAIRINGS INSTALLED, THE CLIMB PERFORMANCE IS REDUCED BY APPROXIMATELY 3%.",
    afm_wheel_fairings_note_cruise:
        "CAUTION — IN AIRPLANE OPERATIONS WITHOUT THE OPTIONAL WHEEL FAIRINGS INSTALLED, THE MAXIMUM CRUISING SPEED IS REDUCED BY APPROXIMATELY 5%.",
    fuel_usable: "Usable fuel (US gal)",
    fuel_reserve: "Reserve (US gal)",
    derived_not_afm: "Derived, not an AFM chart",
    landing_no_corr: "No wind, temperature, or weight correction (Table 4).",
    landing_ground_idle_toggle: "Ground idle set to 1000 RPM",
    landing_ground_idle_factor_prefix: "§5.3.12 NOTE ground idle 1000 RPM: Table 4",
    landing_ground_idle_off: "Off",
    landing_ground_idle_on: "On",
    takeoff: "Takeoff (Fig 5.4)",
    climb: "Climb (via Fig 5.2)",
    cruise: "Cruise",
    landing: "Landing (Table 4)",
    wind_card: "Wind (Fig 5.3)",
    unavailable: "Unavailable",
    derived_time: "Derived time (no AFM chart): ",
    plot_hint: "Drag the cursor on digitized curves (not AFM scans). Pointer and touch supported.",
    plot_readout_chart: "Chart read",
    plot_readout_core: "Core",
    pa_override_chip: "Chart PA",
    pa_override_reset: "Reset PA",
    axis_hw_kt: "HW kt",
    axis_oat_c: "OAT °C",
    axis_base_roll_m: "Base roll m",
    axis_ea_ft: "EA ft",
    axis_roc_fpm: "ROC fpm",
    axis_pa_ft: "PA ft",
    axis_dist_ft: "ft",
    axis_angle_deg: "Angle °",
    axis_weight_kg: "Weight (kg)",
    axis_tailwind_kt: "Tailwind (kt)",
    axis_obstacle_m: "Obstacle (m)",
    axis_dist_m: "m",
    provenance_rev: " rev ",
    cruise_oat: "Cruise OAT (°C)",
    cruise_oat_estimated_label: "Estimated OAT (ISA lapse from ground) (°C)",
    cruise_oat_use_estimate: "Use ground lapse estimate again",
    cruise_disa_readout: "ΔISA (cruise)",
    cruise_isa: "ISA",
    cruise_disa_minus20: "dISA −20",
    cruise_disa_plus10: "dISA +10",
    afm_tag_table: "Table",
    afm_tag_interpolated: "Interpolated",
    table_rows_used_title: "Table rows used",
    table_source_pa_lbl: "PA",
    table_source_pa_weight_lbl: "PA weight",
    table_source_rpm_lbl: "RPM",
    table_source_rpm_sample_lbl: "Sample RPM",
    table_source_rpm_weight_lbl: "RPM weight",
    table_source_landing_roll_ft: "Ground roll (ft)",
    table_source_landing_over_50ft_ft: "Over 50 ft (ft)",
    cruise_tbl_group_disa_m20: "ΔISA −20",
    cruise_tbl_group_isa: "ISA",
    cruise_tbl_group_disa_p10: "ΔISA +10",
    cruise_tbl_pct_bhp: "% BHP",
    cruise_tbl_ktas: "TAS (kt)",
    cruise_tbl_gph: "GPH",
    obstacle_preset_afm: "5 m / 16 ft (AFM)",
    obstacle_preset_screen: "15 m / 50 ft",
    strip_fig54_base: "Fig 5.4 · base",
    strip_fig54_wt: "Wt",
    strip_fig54_hw: "HW",
    strip_fig54_tw: "Tailwind",
    strip_panel_oat_pa: "OAT / PA",
    strip_panel_weight: "Weight",
    strip_panel_hw: "Headwind (kt)",
    strip_panel_tw: "Tailwind (kt)",
    strip_panel_wind: "Headwind / tailwind (kt)",
    strip_panel_obstacle: "Obstacle",
    strip_climb_roc_x_axis: "Rate of climb (fpm)",
    strip_climb_roc_ea_y_axis: "Equivalent altitude (ft)",
    strip_fig52_ea: "Fig 5.2 EA",
    strip_fig55_roc: "Fig 5.5 ROC",
    strip_fig56_roc: "Fig 5.6 ROC",
    strip_fig58_roc: "Fig 5.8 ROC",
    takeoff_ground_roll: "Ground roll",
    takeoff_ground_roll_at_chart_zero: "Ground roll (0 m)",
    takeoff_over_obstacle: "Over obstacle",
    takeoff_signed_headwind: "Along-runway wind (kt, +HW / −TW)",
    takeoff_weight_kg: "Weight (kg)",
    obstacle_ft: "Obstacle height (ft)",
    unit_kg: "kg",
    unit_lb: "lb",
    unit_m: "m",
    unit_ft: "ft",
    takeoff_rounding_label: "Distance rounding",
    takeoff_rounding_raw: "No rounding",
    takeoff_rounding_ceil_10m: "Round up → 10 m",
    takeoff_rounding_ceil_50ft: "Conservative → 50 ft",
    takeoff_sweep_title: "Takeoff sweep",
    takeoff_sweep_run: "Build table",
    takeoff_sweep_var_oat: "OAT (°C)",
    takeoff_sweep_var_hw: "Along-runway wind (kt)",
    takeoff_sweep_var_obs: "Obstacle",
    takeoff_sweep_start: "From",
    takeoff_sweep_end: "To",
    takeoff_sweep_step: "Step",
    takeoff_sweep_col_gr: "Ground roll",
    takeoff_sweep_col_to: "Takeoff",
    takeoff_sweep_col_tag: "Tag",
    takeoff_sweep_oob_footnote: "— = combination off chart (interpolation only, no extrapolation).",
    manual_title: "Methods manual",
    manual_intro: "Each calculation path, AFM source, interpolation, and limitations.",
    afm_link: "Official AFM Rev 29",
    portal_link: "Diamond technical publications",
    amm_link: "AMM DA201-C1 Rev 25",
    methods: &EN_METHODS,
};

const ES_METHODS: [MethodCopy; 10] = [
    MethodCopy {
        title: "Altitud de presión y densidad",
        afm: "AFM DA202-C1 Rev 29 — entradas de performance",
        summary: "Se ingresa altitud de presión (ft) como si el día fuera estándar (1013,25 hPa); sin corrección por QNH. La PA de carta directa (cursor) anula el valor escrito.",
        interpolation: "Fórmula directa; cartas usan interpolación lineal.",
        limitations: "Conversión hPa→ft aproximada; verifique con ejemplos del AFM.",
    },
    MethodCopy {
        title: "Altitud equivalente (Fig 5.2)",
        afm: "AFM p. 5-6, Figura 5.2",
        summary: "Interpolación lineal en malla digitalizada PA × OAT.",
        interpolation: "Lineal en T por fila, luego lineal en PA.",
        limitations: "Requiere malla completa Rev 29.",
    },
    MethodCopy {
        title: "Distancia de despegue (Fig 5.4)",
        afm: "AFM p. 5-9 / 5-10, Figura 5.4",
        summary: "Cadena: rodaje base → factor de peso → panel de viento → obstáculo (m).",
        interpolation: "Lineal en cada panel; aviso si OAT/PA/peso/viento fuera de rango.",
        limitations: "Muestra lectura cruda y valor conservador al alza a 50 ft (m = equivalente ft), p. ej. 1154 → 1200 ft.",
    },
    MethodCopy {
        title: "ROC trepada (Fig 5.5 / 5.6 / 5.8)",
        afm: "AFM p. 5-11, 5-12, 5-16",
        summary: "ROC (fpm) = a + b × EA; figura según flaps/fase seleccionada.",
        interpolation: "EA desde Fig 5.2; coeficientes fijos por figura.",
        limitations: "Tiempo de trepada: estimación derivada, sin carta AFM.",
    },
    MethodCopy {
        title: "Crucero (Tabla 3)",
        afm: "AFM p. 5-14 / 5-15, Tabla 3",
        summary: "ΔISA = OAT − ISA en PA. rmin(PA) lineal; RPM válido [rmin(PA), 2800]. Parámetro s normalizado, RPM_row por fila, mezcla en PA. %BHP: inversión en [rmin(PA), 2800]. Carenados: ~−5% TAS; GPH sin escalar. Filas usadas: «Peso PA» y «Peso RPM» como arriba; «Peso ΔISA» es el peso de cada columna ΔISA dentro de la fila RPM.",
        interpolation: "Tabla o Interpolado. Valor = Σ (Peso PA × Peso RPM × Peso ΔISA × celda) sobre las esquinas usadas. Fuera de carta → sin número («Fuera de carta»).\n\n«Peso» es el peso de la interpolación (no el peso del avión). Peso RPM: qué fracción de tu RPM queda entre dos columnas impresas — p. ej. columnas 2400 y 2500 y vuelas 2450 → 0,5 y 0,5; a 2475 → 2500 lleva 0,75 y 2400 0,25; siempre suman 1. Peso PA (y Peso ΔISA en Tabla 3) funciona igual entre las dos filas impresas alrededor de tu valor.\n\nEjemplo: a mitad de camino en RPM y un cuarto del camino en PA: 0,5×0,75×(PA baja, RPM bajo) + 0,5×0,75×(PA baja, RPM alto) + 0,5×0,25×(PA alta, RPM bajo) + 0,5×0,25×(PA alta, RPM alto).\n\nEs interpolación lineal ordinaria en ambas direcciones; la app muestra los pesos para poder repetirlo a mano con las filas impresas resaltadas.",
        limitations: "GPH transcritos tal como impreso (anomalías conocidas).",
    },
    MethodCopy {
        title: "Crucero MT 175 R (AFMS E-1487 Fig 1–4)",
        afm: "AFMS E-1487 Fig 4 (suplemento MT 175 R)",
        summary: "Altitud de densidad ISA troposférica exacta: σ = (T_std/T₀)^5.25588 × (T₀/T_real) con T_std = 288,15 K − 0,0019812 °C/ft × PA; DA = (1 − σ^(1/4.25588)) × T₀ / 0,0019812 (misma función que da20-core en cruise MT). Consulta Fig 4 en esa DA y RPM.",
        interpolation: "Tabla o Interpolado. Valor = Σ (Peso DA × Peso RPM × celda) sobre las esquinas usadas (cada par de pesos en DA o RPM suma 1).\n\n«Peso» es el peso de la interpolación. Peso RPM: entre columnas impresas — p. ej. 2400 y 2500 a 2450 → 0,5 y 0,5; a 2475 → 0,75 y 0,25; siempre suman 1. Peso DA igual entre filas impresas alrededor de tu altitud de densidad.\n\nEjemplo: mitad en RPM y un cuarto en DA: 0,5×0,75×(DA baja, RPM bajo) + 0,5×0,75×(DA baja, RPM alto) + 0,5×0,25×(DA alta, RPM bajo) + 0,5×0,25×(DA alta, RPM alto).\n\nInterpolación lineal en ambas direcciones; la app muestra los pesos para repetirlo a mano desde las filas resaltadas.",
        limitations: "Fuera del casco Fig 4 → «Fuera de carta». Despegue/trepada/aterrizaje sin cambio por hélice MT (AFMS E-1487).",
    },
    MethodCopy {
        title: "Aterrizaje (Tabla 4)",
        afm: "AFM p. 5-17, Tabla 4",
        summary: "Consulta por altitud de presión únicamente (ft). Opcional: NOTA §5.3.12 ralentí en tierra 1000 RPM → rodaje ×1,07 y distancia 50 ft ×1,05 después de la interpolación Tabla 4.",
        interpolation: "Lineal en altitudes de la tabla. Filas usadas sin factor de ralentí.",
        limitations: "Sin corrección por viento, temperatura ni peso salvo la NOTA de ralentí 1000 RPM. 1360 ft ≈ 415 m; texto AFM cita 414 m.",
    },
    MethodCopy {
        title: "Componentes de viento (Fig 5.3)",
        afm: "AFM p. 5-8, Figura 5.3",
        summary: "Viento de frente y cruzado por trigonometría; formato 200 / 08 kt.",
        interpolation: "N/A para componentes.",
        limitations: "Rumbo pista = número × 10°; aviso si pista no numérica.",
    },
    MethodCopy {
        title: "OAT de crucero desde tierra",
        afm: "Planificación UI (no carta AFM)",
        summary: "En Crucero y Autonomía, «Tierra / Aeródromo» es independiente en cada ficha: elevación y OAT en tierra solo alimentan la estimación de OAT de crucero en esa misma ficha (OAT_alt = OAT_tierra − 1,98 °C/1000 ft × (PA_crucero − elevación)). Despegue, subida y aterrizaje guardan su propia PA y OAT. ΔISA mostrado: OAT_crucero − (15 − 1,98 × PA/1000).",
        interpolation: "Aproximación de gradiente estándar; el usuario puede fijar OAT de crucero manualmente y volver a la estimación.",
        limitations: "En la vida real el gradiente varía e inversiones térmicas son frecuentes (p. ej. Santiago); la estimación no sustituye METAR/sondas. El cálculo de performance sigue las reglas interpolate-only y «Fuera de carta» del núcleo.",
    },
    MethodCopy {
        title: "Autonomía y alcance (derivado)",
        afm: "AFM §1.8, §5.3.10 (duración máxima) + Tabla 3 GPH",
        summary: "(Combustible utilizable − reserva) / caudal Tabla 3; alcance = autonomía × TAS.",
        interpolation: "Hereda el casco Tabla 3; fuera de carta si falla el crucero.",
        limitations: "Derivado, no carta AFM. Combustible utilizable 24,0 US gal (total 24,5; AFM §1.8). Placas: 21,3 / 19,8 gal. Ver AFM §5.3.10 duración máxima.",
    },
];

const EN_METHODS: [MethodCopy; 10] = [
    MethodCopy {
        title: "Pressure & density altitude",
        afm: "AFM DA202-C1 Rev 29 — performance inputs",
        summary:
            "Pressure altitude (ft) is entered as for a standard day (1013.25 hPa); no QNH correction. Direct chart PA from the cursor overrides the typed value.",
        interpolation: "Direct formula; charts use linear interpolation.",
        limitations: "Approximate hPa→ft conversion.",
    },
    MethodCopy {
        title: "Equivalent altitude (Fig 5.2)",
        afm: "AFM p. 5-6, Figure 5.2",
        summary: "Linear interpolation on digitized PA × OAT grid.",
        interpolation: "Linear on T per row, then linear on PA.",
        limitations: "Requires full Rev 29 grid.",
    },
    MethodCopy {
        title: "Takeoff distance (Fig 5.4)",
        afm: "AFM p. 5-9 / 5-10, Figure 5.4",
        summary: "Chain: base roll → weight factor → wind panel → obstacle (m).",
        interpolation: "Linear on each panel; warning if OAT/PA/weight/wind out of range.",
        limitations: "Shows raw chart chain and conservative round-up to next 50 ft (m from ft), e.g. 1154 → 1200 ft.",
    },
    MethodCopy {
        title: "Climb ROC (Figs 5.5 / 5.6 / 5.8)",
        afm: "AFM p. 5-11, 5-12, 5-16",
        summary: "ROC (fpm) = a + b × EA; figure from selected flap/phase.",
        interpolation: "EA from Fig 5.2; fixed coefficients per figure.",
        limitations: "Climb time is a derived estimate — no AFM chart.",
    },
    MethodCopy {
        title: "Cruise (Table 3)",
        afm: "AFM p. 5-14 / 5-15, Table 3",
        summary: "ΔISA = OAT − ISA at PA. Interpolation only: rmin(PA) linear between each row's first RPM; valid RPM is [rmin(PA), 2800]. Normalize s, interpolate each row at RPM_row, blend in PA. Set %BHP inverts on [rmin(PA), 2800]. Wheel fairings: ~−5% TAS; GPH not scaled. Row weights: «PA weight» and «RPM weight» as above; «ΔISA weight» is the blend weight for each ΔISA column within the RPM row.",
        interpolation: "Table or Interpolated only. Result = Σ (PA weight × RPM weight × ΔISA weight × cell) over used corners. Off chart → no numeric result.\n\n\"Weight\" means the interpolation weight (not aircraft weight). RPM weight: how far your RPM sits between two printed RPM columns — e.g. columns 2400 and 2500 and you fly 2450, each gets 0.5; at 2475, 2500 gets 0.75 and 2400 gets 0.25; the two always sum to 1. PA weight (and ΔISA weight on Table 3) works the same for the two printed rows around your value.\n\nExample halfway in RPM and a quarter of the way in PA: 0.5×0.75×(low PA, low RPM) + 0.5×0.75×(low PA, high RPM) + 0.5×0.25×(high PA, low RPM) + 0.5×0.25×(high PA, high RPM).\n\nThis is ordinary linear interpolation in both directions; the app shows the weights so you can redo it by hand from the highlighted printed rows.",
        limitations: "GPH as printed (known oddities).",
    },
    MethodCopy {
        title: "Cruise MT 175 R (AFMS E-1487 Fig 1–4)",
        afm: "AFMS E-1487 Fig 4 (MT 175 R supplement)",
        summary: "Exact ISA troposphere density altitude: σ = (T_std/T₀)^5.25588 × (T₀/T_actual) with T_std = 288.15 K − 0.0019812 °C/ft × PA; DA = (1 − σ^(1/4.25588)) × T₀ / 0.0019812 (same as da20-core MT cruise). Fig 4 lookup at that DA and RPM.",
        interpolation: "Table or Interpolated only. Result = Σ (DA weight × RPM weight × cell) over used corners (each bracketing weight pair sums to 1).\n\n\"Weight\" means the interpolation weight. RPM weight: between printed columns — e.g. 2400 and 2500 at 2450, each gets 0.5; at 2475, 0.75 and 0.25; always sum to 1. DA weight works the same between printed rows around your density altitude.\n\nExample halfway in RPM and a quarter of the way in DA: 0.5×0.75×(low DA, low RPM) + 0.5×0.75×(low DA, high RPM) + 0.5×0.25×(high DA, low RPM) + 0.5×0.25×(high DA, high RPM).\n\nOrdinary linear interpolation in both directions; the app shows the weights so you can redo it by hand from the highlighted rows.",
        limitations: "Outside Fig 4 hull → Out of chart. Takeoff/climb/landing unchanged by MT propeller (AFMS E-1487).",
    },
    MethodCopy {
        title: "Landing (Table 4)",
        afm: "AFM p. 5-17, Table 4",
        summary: "Pressure-altitude lookup only (ft). Optional: §5.3.12 NOTE ground idle 1000 RPM → ground roll ×1.07 and 50 ft distance ×1.05 after Table 4 interpolation.",
        interpolation: "Linear on table altitudes. Source rows show uncorrected table values.",
        limitations:
            "No wind, temperature, or weight correction except the 1000 RPM ground-idle NOTE. 1360 ft ≈ 415 m; AFM text cites 414 m.",
    },
    MethodCopy {
        title: "Wind components (Fig 5.3)",
        afm: "AFM p. 5-8, Figure 5.3",
        summary: "Headwind/crosswind by trigonometry; display 200 / 08 kt.",
        interpolation: "N/A for components.",
        limitations: "Runway heading: three-digit magnetic 001–360 only; 1–2 digits flagged as ambiguous.",
    },
    MethodCopy {
        title: "Cruise OAT from ground",
        afm: "UI planning layer (not an AFM chart)",
        summary: "On Cruise and Endurance, Ground / Aerodrome is independent per tab: ground elevation and OAT only feed the cruise OAT lapse estimate on the same tab (OAT_alt = OAT_ground − 1.98 °C per 1000 ft × (cruise_PA − elevation)). Takeoff, climb, and landing keep separate PA and OAT. Displayed ΔISA: cruise_OAT − (15 − 1.98 × PA/1000).",
        interpolation: "Standard lapse estimate; the user can override cruise OAT manually and revert to the estimate.",
        limitations: "Real lapse rates vary and inversions are common (e.g. Santiago); the estimate is not a substitute for METAR/soundings. Downstream performance still uses interpolate-only rules and Out of chart from the core.",
    },
    MethodCopy {
        title: "Endurance & range (derived)",
        afm: "AFM §1.8, §5.3.10 (Maximum Flight Duration) + Table 3 GPH",
        summary: "(Usable fuel − reserve) / Table 3 flow; range = endurance × TAS.",
        interpolation: "Inherits Table 3 cruise hull; out of chart if cruise fails.",
        limitations: "Derived, not an AFM chart. Usable fuel 24.0 US gal (24.5 total per AFM §1.8). Placards: 21.3 / 19.8 gal. See AFM §5.3.10.",
    },
];

pub fn format_obstacle_over(locale: Locale, height_m: f64) -> String {
    let c = copy(locale);
    let ft = height_m * 3.280839895;
    format!(
        "{} {:.1} m / {:.0} ft",
        c.takeoff_over_obstacle, height_m, ft
    )
}

pub fn format_landing_ground_idle_breakdown(
    locale: Locale,
    roll_chart_ft: i64,
    over_chart_ft: i64,
    roll_ft: i64,
    over_ft: i64,
) -> String {
    let c = copy(locale);
    let factors = match locale {
        Locale::Es => "×1,07 / ×1,05",
        Locale::En => "×1.07 / ×1.05",
    };
    format!(
        "{} {roll_chart_ft}/{over_chart_ft} ft → {factors} → {roll_ft}/{over_ft} ft",
        c.landing_ground_idle_factor_prefix,
    )
}

pub fn format_takeoff_over_obstacle_result(locale: Locale, ft: f64, m: f64) -> String {
    let ft_i = ft.round() as i64;
    let m_i = m.round() as i64;
    let _ = locale;
    format!("{ft_i} ft / {m_i} m")
}

pub fn format_takeoff_conservative_estimate(locale: Locale, ft: f64, m: f64) -> String {
    let ft_i = ft.round() as i64;
    let m_i = m.round() as i64;
    match locale {
        Locale::Es => format!(
            "Estimación conservadora (redondeo al alza al siguiente 50 ft): {ft_i} ft / {m_i} m"
        ),
        Locale::En => {
            format!("Conservative estimate (rounded up to next 50 ft): {ft_i} ft / {m_i} m")
        }
    }
}

pub fn format_strip_chart_roll_obs(locale: Locale, roll_m: f64, obs_m: f64) -> String {
    match locale {
        Locale::Es => format!(
            "{:.0} m rodaje / {:.0} m obst.",
            roll_m.round(),
            obs_m.round()
        ),
        Locale::En => format!("{:.0} m roll / {:.0} m obs", roll_m.round(), obs_m.round()),
    }
}

/// Strip PA guide label (OAT/PA takeoff panel and Fig 5.2 PA-curve trace) — ES: **AP**, EN: **PA**.
pub fn format_takeoff_strip_pa_trace_label(locale: Locale, pa_ft: f64) -> String {
    let ft = pa_ft.round();
    match locale {
        Locale::Es => format!("AP {ft:.0} ft"),
        Locale::En => format!("PA {ft:.0} ft"),
    }
}

/// Saved readout / phase summary: chart ROC, penalty tag, optional second line for −3% value.
pub fn format_climb_readout_summary(
    locale: Locale,
    penalized_roc_fpm: f64,
    chart_roc_fpm: Option<f64>,
    fairings_installed: bool,
    afm_tag: AfmLookupTag,
) -> String {
    let c = copy(locale);
    let adj = penalized_roc_fpm.round() as i64;
    let chart = chart_roc_fpm.map(|x| x.round() as i64).unwrap_or(adj);
    let tag = afm_lookup_tag_label(locale, afm_tag);
    if fairings_installed {
        return format!("{chart} fpm · {tag}");
    }
    let suffix = c.climb_roc_fairing_penalty_suffix;
    if chart_roc_fpm.is_none() {
        return format!("{adj} fpm {suffix} · {tag}");
    }
    format!(
        "{chart} fpm {suffix} · {tag}\n{} {adj} fpm",
        c.climb_roc_adjusted_label
    )
}

/// Interactive chart cursor line: chart ROC (unpenalized) vs penalized when fairings off.
pub fn format_climb_readout_interactive_pair(
    locale: Locale,
    penalized_roc_fpm: f64,
    chart_roc_fpm: Option<f64>,
    fairings_installed: bool,
    afm_tag: AfmLookupTag,
) -> (String, String) {
    let c = copy(locale);
    let adj = penalized_roc_fpm.round() as i64;
    let chart = chart_roc_fpm.map(|x| x.round() as i64).unwrap_or(adj);
    let tag = afm_lookup_tag_label(locale, afm_tag);
    if fairings_installed {
        let s = format!("{chart} fpm · {tag}");
        return (s.clone(), s);
    }
    let suffix = c.climb_roc_fairing_penalty_suffix;
    let chart_str = format!("{chart} fpm {suffix} · {tag}");
    let core_str = format!("{adj} fpm");
    (chart_str, core_str)
}

/// Saved readout / list: Table 3 TAS (chart vs −5%) and unscaled GPH.
pub fn format_cruise_readout_summary(
    locale: Locale,
    penalized_tas_kt: f64,
    chart_tas_kt: Option<f64>,
    fairings_installed: bool,
    gph: f64,
    pct_bhp: Option<f64>,
    afm_tag: AfmLookupTag,
) -> String {
    let c = copy(locale);
    let pen = penalized_tas_kt.round() as i64;
    let chart = chart_tas_kt.map(|x| x.round() as i64).unwrap_or(pen);
    let tag = afm_lookup_tag_label(locale, afm_tag);
    let bhp = pct_bhp
        .map(|p| format!(" · {:.2}% BHP", p))
        .unwrap_or_default();
    if fairings_installed {
        return format!("{chart} kt TAS · {gph:.1} gph{bhp} · {tag}");
    }
    let suffix = c.cruise_tas_fairing_penalty_suffix;
    if chart_tas_kt.is_none() {
        return format!("{pen} kt TAS {suffix} · {gph:.1} gph{bhp} · {tag}");
    }
    format!(
        "{chart} kt TAS {suffix} · {gph:.1} gph{bhp} · {tag}\n{} {pen} kt TAS",
        c.cruise_tas_adjusted_label
    )
}

/// Endurance/range line: uses chart TAS for “raw” NM when fairings off; penalized TAS on adjusted line.
#[allow(clippy::too_many_arguments)]
pub fn format_endurance_readout_summary(
    locale: Locale,
    endurance_h: f64,
    range_pen_nm: f64,
    range_chart_nm: Option<f64>,
    tas_pen: f64,
    tas_chart: Option<f64>,
    gph: f64,
    fairings_installed: bool,
    afm_tag: AfmLookupTag,
) -> String {
    let c = copy(locale);
    let h = endurance_h;
    let g = gph;
    let tag = afm_lookup_tag_label(locale, afm_tag);
    if fairings_installed {
        let t = tas_chart.unwrap_or(tas_pen).round() as i64;
        let r = range_chart_nm.unwrap_or(range_pen_nm).round() as i64;
        return format!("{h:.1} h · {r} NM · {g:.1} gph @ {t} kt · {tag}");
    }
    let suffix = c.cruise_tas_fairing_penalty_suffix;
    let tc = tas_chart
        .map(|x| x.round() as i64)
        .unwrap_or(tas_pen.round() as i64);
    let rc = range_chart_nm
        .map(|x| x.round() as i64)
        .unwrap_or(range_pen_nm.round() as i64);
    let tp = tas_pen.round() as i64;
    let rp = range_pen_nm.round() as i64;
    if tas_chart.is_none() || range_chart_nm.is_none() {
        return format!("{h:.1} h · {rp} NM · {g:.1} gph @ {tp} kt {suffix} · {tag}");
    }
    format!(
        "{h:.1} h · {rc} NM · {g:.1} gph @ {tc} kt {suffix} · {tag}\n{} {rp} NM @ {tp} kt",
        c.endurance_range_adjusted_label
    )
}

#[allow(dead_code)]
pub fn phase_slug_label(locale: Locale, slug: &str) -> String {
    let c = copy(locale);
    match slug {
        "takeoff" => c.tab_takeoff.to_string(),
        "climb" => c.tab_climb.to_string(),
        "cruise" => c.tab_cruise.to_string(),
        "endurance" => c.tab_endurance.to_string(),
        "landing" => c.tab_landing.to_string(),
        "method" | "manual" => c.tab_method.to_string(),
        other => other.to_string(),
    }
}

pub fn format_along_runway_component(
    locale: Locale,
    components: &da20_core::types::WindComponents,
) -> Option<String> {
    use da20_core::wind::{along_runway_display_kt, format_component_kt};
    let c = copy(locale);
    let (is_hw, kt) = along_runway_display_kt(components);
    if kt < 0.05 {
        return None;
    }
    Some(if is_hw {
        format!("{} {} kt", c.wind_head, format_component_kt(kt))
    } else {
        format!("{} {} kt", c.wind_tail, format_component_kt(kt))
    })
}

pub fn format_crosswind_component(locale: Locale, crosswind_kt: f64) -> String {
    if crosswind_kt.abs() < 0.05 {
        return String::new();
    }
    use da20_core::wind::format_component_kt;
    let mag = format_component_kt(crosswind_kt.abs());
    match locale {
        Locale::Es if crosswind_kt >= 0.0 => format!("Cruzado {mag} kt desde la derecha"),
        Locale::Es => format!("Cruzado {mag} kt desde la izquierda"),
        Locale::En if crosswind_kt >= 0.0 => format!("Crosswind {mag} kt from the right"),
        Locale::En => format!("Crosswind {mag} kt from the left"),
    }
}

pub fn data_status_label(locale: Locale, status: &str) -> String {
    match (locale, status) {
        (Locale::Es, "production") => "Producción".into(),
        (Locale::Es, "draft") => "Borrador".into(),
        (Locale::En, "production") => "Production".into(),
        (Locale::En, "draft") => "Draft".into(),
        (_, other) => other.to_string(),
    }
}

pub fn format_out_of_chart(
    locale: Locale,
    ooc: &da20_core::chart_bounds::ChartOutOfRange,
) -> String {
    warn_text(locale, &ooc.warning())
}

pub fn warn_text(locale: Locale, w: &da20_core::types::CalculationWarning) -> String {
    if !w.message.is_empty() {
        return w.message.clone();
    }
    let c = copy(locale);
    let ctx = w.args.first().map(String::as_str).unwrap_or("");
    let a1 = w.args.get(1).map(String::as_str).unwrap_or("");
    let _a2 = w.args.get(2).map(String::as_str).unwrap_or("");
    match w.code.as_str() {
        "ENDURANCE_DERIVED" => c.derived_not_afm.to_string(),
        "AFM_FUEL_CAPACITY" => match locale {
            Locale::Es => {
                "AFM §1.8: 24,5 gal total, 24,0 utilizable; placas 21,3 / 19,8 gal. §5.3.10.".into()
            }
            Locale::En => {
                "AFM §1.8: 24.5 gal total, 24.0 usable; placards 21.3 / 19.8 gal. See §5.3.10."
                    .into()
            }
        },
        "LANDING_TABLE_50FT" => match locale {
            Locale::Es => "Tabla 4: 1360 ft (≈415 m); texto AFM cita 414 m.".into(),
            Locale::En => "Table 4: 1360 ft (≈415 m); AFM narrative cites 414 m.".into(),
        },
        "LANDING_NO_WIND" => c.landing_no_corr.to_string(),
        "LANDING_GROUND_IDLE_1000" => match locale {
            Locale::Es => {
                "AFM §5.3.12: ralentí 1000 RPM — rodaje ×1,07, distancia 50 ft ×1,05.".into()
            }
            Locale::En => {
                "AFM §5.3.12: 1000 RPM ground idle — ground roll ×1.07, 50 ft distance ×1.05."
                    .into()
            }
        },
        "DERIVED_ESTIMATE" => match locale {
            Locale::Es => "Estimación derivada de trepada (sin carta AFM).".into(),
            Locale::En => "Derived climb estimate (no AFM chart).".into(),
        },
        "UNITS" => match locale {
            Locale::Es => "Despegue: carta en m, pantalla en ft.".into(),
            Locale::En => "Takeoff: chart in m, display in ft.".into(),
        },
        "TABLE3_GPH" => match locale {
            Locale::Es => "GPH Tabla 3 transcritos del AFM Rev 29.".into(),
            Locale::En => "Table 3 GPH transcribed from AFM Rev 29.".into(),
        },
        "FIG4_GPH" => match locale {
            Locale::Es => "GPH Fig 4 transcritos del AFMS E-1487.".into(),
            Locale::En => "Fig 4 GPH transcribed from AFMS E-1487.".into(),
        },
        "MT_DOUBTFUL_GPH" => {
            let da = w.args.first().map(String::as_str).unwrap_or("");
            let rpm = w.args.get(1).map(String::as_str).unwrap_or("");
            match locale {
                Locale::Es => {
                    format!("Valor de consumo impreso dudoso en el AFMS (AD {da} ft / RPM {rpm}).")
                }
                Locale::En => {
                    format!("Doubtful printed fuel value in AFMS (DA {da} ft / RPM {rpm}).")
                }
            }
        }
        "CRUISE_BHP_OUT_OF_RANGE" => {
            let pct = w.args.first().map(String::as_str).unwrap_or("");
            match locale {
                Locale::Es => format!(
                    "%BHP solicitado {pct}%: RPM resuelto fuera de carta (fuera de la banda ±100 RPM de la Tabla 3 en esta PA/ΔISA)."
                ),
                Locale::En => format!(
                    "Requested %BHP {pct}%: solved RPM is off chart (outside ±100 RPM of Table 3 rows at this PA/ΔISA)."
                ),
            }
        }
        "CHART_OUT_OF_RANGE" => {
            let axis = w.args.first().map(String::as_str).unwrap_or("");
            let min = w.args.get(1).map(String::as_str).unwrap_or("");
            let max = w.args.get(2).map(String::as_str).unwrap_or("");
            match locale {
                Locale::Es => format!("Fuera de carta — {axis} ({min}..{max})."),
                Locale::En => format!("Out of chart — {axis} ({min}..{max})."),
            }
        }
        "CRUISE_BHP_NOT_ACHIEVABLE" => {
            let pct = w.args.first().map(String::as_str).unwrap_or("");
            match locale {
                Locale::Es => format!(
                    "%BHP solicitado {pct}% no alcanzable en esta PA/ΔISA (±100 RPM sobre carta)."
                ),
                Locale::En => format!(
                    "Requested %BHP {pct}% not achievable at this PA/ΔISA (±100 RPM beyond table rows)."
                ),
            }
        }
        "RUNWAY_HEADING" => match locale {
            Locale::Es => format!(
                "Rumbo de pista «{ctx}» inválido — use 001–360 (3 dígitos); cálculo con 0°."
            ),
            Locale::En => format!(
                "Runway heading «{ctx}» invalid — use 001–360 (3 digits); calculation uses 0°."
            ),
        },
        "RUNWAY_HEADING_AMBIGUOUS" => match locale {
            Locale::Es => format!(
                "Rumbo «{ctx}» ambiguo — use 3 dígitos (p. ej. 019, no 19); cálculo con 0°."
            ),
            Locale::En => format!(
                "Heading «{ctx}» ambiguous — use 3 digits (e.g. 019, not 19); calculation uses 0°."
            ),
        },
        "TAILWIND" => match locale {
            Locale::Es => format!("Cola {a1} kt."),
            Locale::En => format!("Tailwind {a1} kt."),
        },
        "TAILWIND_CHART_LIMIT" => {
            let max_kt = w.args.first().map(String::as_str).unwrap_or("");
            let tw_kt = w.args.get(1).map(String::as_str).unwrap_or("");
            match locale {
                Locale::Es => {
                    format!("Cola {tw_kt} kt fuera de carta (máx {max_kt} kt); sin extrapolar.")
                }
                Locale::En => format!(
                    "Tailwind {tw_kt} kt exceeds chart (max {max_kt} kt); not extrapolated."
                ),
            }
        }
        "HEADWIND_CHART_LIMIT" => {
            let max_kt = w.args.first().map(String::as_str).unwrap_or("");
            let hw_kt = w.args.get(1).map(String::as_str).unwrap_or("");
            match locale {
                Locale::Es => format!(
                    "Viento de frente {hw_kt} kt fuera de carta (máx {max_kt} kt); sin extrapolar."
                ),
                Locale::En => format!(
                    "Headwind {hw_kt} kt exceeds chart (max {max_kt} kt); not extrapolated."
                ),
            }
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod i18n_display_tests {
    use super::*;
    use da20_core::types::FlightInputs;
    use da20_core::types::WindComponents;
    use da20_core::wind::wind_chart_panel_needs_crosswind_note;

    #[test]
    fn html_lang_follows_locale() {
        assert_eq!(Locale::Es.html_lang(), "es");
        assert_eq!(Locale::En.html_lang(), "en");
    }

    #[test]
    fn crosswind_line_hidden_when_near_zero() {
        assert!(format_crosswind_component(Locale::Es, 0.0).is_empty());
        assert!(format_crosswind_component(Locale::En, 0.04).is_empty());
    }

    #[test]
    fn along_runway_line_hidden_when_no_along_component() {
        let comp = WindComponents {
            headwind_kt: 0.0,
            crosswind_kt: 8.0,
            along_runway_headwind_kt: 0.0,
        };
        assert!(format_along_runway_component(Locale::Es, &comp).is_none());
    }

    #[test]
    fn tailwind_chart_limit_message_includes_speed_and_max() {
        let w = da20_core::types::CalculationWarning::with_args(
            "TAILWIND_CHART_LIMIT",
            vec!["5.0".into(), "6.0".into()],
        );
        assert_eq!(
            warn_text(Locale::Es, &w),
            "Cola 6.0 kt fuera de carta (máx 5.0 kt); sin extrapolar."
        );
        assert_eq!(
            warn_text(Locale::En, &w),
            "Tailwind 6.0 kt exceeds chart (max 5.0 kt); not extrapolated."
        );
    }

    #[test]
    fn takeoff_strip_pa_trace_label_es_en() {
        assert_eq!(
            format_takeoff_strip_pa_trace_label(Locale::Es, 2500.0),
            "AP 2500 ft"
        );
        assert_eq!(
            format_takeoff_strip_pa_trace_label(Locale::En, 2500.0),
            "PA 2500 ft"
        );
    }

    #[test]
    fn pressure_altitude_input_label_es_en_distinct() {
        let es = copy(Locale::Es);
        let en = copy(Locale::En);
        assert_eq!(es.pressure_altitude_ft, "Altitud de presión (ft)");
        assert_eq!(en.pressure_altitude_ft, "Pressure altitude (ft)");
    }

    #[test]
    fn typed_near_pure_crosswind_triggers_strip_note() {
        let inputs = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 280.0,
            wind_speed_kt: 10.0,
            ..Default::default()
        };
        assert!(wind_chart_panel_needs_crosswind_note(&inputs));
    }

    #[test]
    fn cruise_mode_toggle_i18n_both_locales() {
        let es = copy(Locale::Es);
        let en = copy(Locale::En);
        assert_eq!(es.cruise_mode_set_rpm, "Fijar RPM");
        assert_eq!(es.cruise_mode_set_bhp, "Fijar %BHP");
        assert_eq!(en.cruise_mode_set_rpm, "Set RPM");
        assert_eq!(en.cruise_mode_set_bhp, "Set %BHP");
    }

    #[test]
    fn climb_flap_toggle_placard_labels_same_es_en_three_options() {
        const TO: &str = "Flaps T/O";
        const UP: &str = "Flaps UP/CRUISE";
        const LDG: &str = "Flaps LDG - Balked Landing";
        let es = copy(Locale::Es);
        let en = copy(Locale::En);
        assert_eq!(es.climb_flaps_to, TO);
        assert_eq!(es.climb_flaps_up, UP);
        assert_eq!(es.climb_flaps_ldg, LDG);
        assert_eq!(en.climb_flaps_to, TO);
        assert_eq!(en.climb_flaps_up, UP);
        assert_eq!(en.climb_flaps_ldg, LDG);
        assert_ne!(TO, UP);
        assert_ne!(UP, LDG);
    }

    #[test]
    fn method_es_table3_interpolation_worked_weights_and_delta_isa_sigma() {
        let entry = ES_METHODS
            .iter()
            .find(|m| m.title.starts_with("Crucero (Tabla 3)"))
            .expect("Tabla 3");
        assert!(
            entry.interpolation.contains("Peso ΔISA"),
            "must name ΔISA weight inside Σ product"
        );
        assert!(entry.interpolation.contains("2400"));
        assert!(entry.interpolation.contains("0,5"));
        assert!(entry.interpolation.contains("0,75"));
        assert!(!entry.interpolation.contains("Σ Peso PA × Σ Peso RPM"));
    }

    #[test]
    fn method_es_mt_density_formula_shows_5_25588() {
        let entry = ES_METHODS
            .iter()
            .find(|m| m.title.contains("MT 175"))
            .expect("MT");
        assert!(entry.summary.contains("5.25588"));
        assert!(!entry.summary.contains("5,256"));
    }

    #[test]
    fn method_es_cruise_ground_not_shared_with_takeoff_wording() {
        let entry = ES_METHODS
            .iter()
            .find(|m| m.title.contains("OAT de crucero"))
            .expect("ground OAT");
        assert!(!entry.summary.contains("Parámetros compartidos"));
        assert!(!entry.summary.contains("En despegue alimentan"));
        assert!(entry.summary.contains("independiente en cada ficha"));
    }
}
