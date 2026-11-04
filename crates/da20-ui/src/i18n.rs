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
    pub tab_readings: &'static str,
    pub tab_manual: &'static str,
    pub lang_toggle: &'static str,
    pub theme_toggle: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub subtitle: &'static str,
    pub inputs_title: &'static str,
    pub results_title: &'static str,
    pub save_reading: &'static str,
    pub export_csv: &'static str,
    pub export_json: &'static str,
    pub field_elev: &'static str,
    pub qnh: &'static str,
    pub qnh_dual: &'static str,
    pub oat: &'static str,
    pub runway: &'static str,
    pub wind: &'static str,
    pub weight: &'static str,
    pub cruise_pa: &'static str,
    pub cruise_rpm: &'static str,
    pub obstacle_m: &'static str,
    pub climb_figure: &'static str,
    pub climb_fig_55: &'static str,
    pub climb_fig_56: &'static str,
    pub climb_fig_58: &'static str,
    pub climb_flaps_to: &'static str,
    pub climb_flaps_up: &'static str,
    pub fuel_usable: &'static str,
    pub fuel_reserve: &'static str,
    pub derived_not_afm: &'static str,
    pub landing_no_corr: &'static str,
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
    pub readings_compare: &'static str,
    pub compare_pick_a: &'static str,
    pub compare_pick_b: &'static str,
    pub axis_hw_kt: &'static str,
    pub axis_oat_c: &'static str,
    pub axis_base_roll_m: &'static str,
    pub axis_ea_ft: &'static str,
    pub axis_roc_fpm: &'static str,
    pub axis_pa_ft: &'static str,
    pub axis_dist_ft: &'static str,
    pub axis_angle_deg: &'static str,
    pub provenance_rev: &'static str,
    pub cruise_oat: &'static str,
    pub cruise_isa: &'static str,
    pub cruise_disa_minus20: &'static str,
    pub cruise_disa_plus10: &'static str,
    pub obstacle_preset_afm: &'static str,
    pub obstacle_preset_screen: &'static str,
    pub strip_fig54_base: &'static str,
    pub strip_fig54_wt: &'static str,
    pub strip_fig54_hw: &'static str,
    pub strip_panel_oat_pa: &'static str,
    pub strip_panel_weight: &'static str,
    pub strip_panel_hw: &'static str,
    pub strip_panel_obstacle: &'static str,
    pub strip_fig52_ea: &'static str,
    pub strip_fig55_roc: &'static str,
    pub strip_fig56_roc: &'static str,
    pub strip_fig58_roc: &'static str,
    pub takeoff_ground_roll: &'static str,
    pub takeoff_over_obstacle: &'static str,
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

const ES: Copy = Copy {
    disclaimer: "No es fuente oficial. No aprobado por Diamond Aircraft ni DGAC. Valores digitalizados y aproximados. El AFM aprobado rige toda la planificación de vuelo.",
    logo_suffix: "Rendimiento",
    tab_takeoff: "Despegue",
    tab_climb: "Ascenso",
    tab_cruise: "Crucero y potencia",
    tab_endurance: "Autonomía y alcance",
    tab_landing: "Aterrizaje",
    tab_readings: "Lecturas",
    tab_manual: "Manual",
    lang_toggle: "Idioma",
    theme_toggle: "Tema",
    theme_system: "Sistema",
    theme_light: "Claro",
    theme_dark: "Oscuro",
    subtitle: "AFM DA202-C1 Rev 29 (30 jun 2021) — Capítulo 5 digitalizado",
    inputs_title: "Entradas",
    results_title: "Resultados",
    save_reading: "Guardar lectura",
    export_csv: "Exportar CSV",
    export_json: "Exportar JSON",
    field_elev: "Elevación del aeródromo (ft)",
    qnh: "QNH",
    qnh_dual: "QNH (hPa / inHg)",
    oat: "OAT (°C)",
    runway: "Pista",
    wind: "Viento (° / kt)",
    weight: "Peso (lb)",
    cruise_pa: "PA crucero (ft)",
    cruise_rpm: "RPM crucero",
    obstacle_m: "Altura obstáculo (m)",
    climb_figure: "Configuración de trepada (AFM)",
    climb_fig_55: "Fig 5.5 — flaps crucero",
    climb_fig_56: "Fig 5.6 — flaps despegue",
    climb_fig_58: "Fig 5.8 — aterrizaje abortado",
    climb_flaps_to: "Flaps DES",
    climb_flaps_up: "Flaps UP",
    fuel_usable: "Combustible utilizable (US gal)",
    fuel_reserve: "Reserva (US gal)",
    derived_not_afm: "Derivado, no es carta AFM",
    landing_no_corr: "Sin corrección por viento, temperatura ni peso (Tabla 4).",
    takeoff: "Despegue (Fig 5.4)",
    climb: "Trepada (via Fig 5.2)",
    cruise: "Crucero (Tabla 3)",
    landing: "Aterrizaje (Tabla 4)",
    wind_card: "Viento (Fig 5.3)",
    unavailable: "No disponible",
    derived_time: "Tiempo estimado (sin carta AFM): ",
    plot_hint: "Arrastre el cursor sobre las curvas digitalizadas (no son copias del AFM). Compatible con puntero y táctil.",
    plot_readout_chart: "Lectura carta",
    plot_readout_core: "Resultado núcleo",
    pa_override_chip: "PA carta",
    pa_override_reset: "Restablecer PA",
    readings_compare: "Comparar lecturas",
    compare_pick_a: "Lectura A",
    compare_pick_b: "Lectura B",
    axis_hw_kt: "HW kt",
    axis_oat_c: "OAT °C",
    axis_base_roll_m: "Rodaje base m",
    axis_ea_ft: "EA ft",
    axis_roc_fpm: "ROC fpm",
    axis_pa_ft: "PA ft",
    axis_dist_ft: "ft",
    axis_angle_deg: "Ángulo °",
    provenance_rev: " rev ",
    cruise_oat: "OAT crucero (°C)",
    cruise_isa: "ISA",
    cruise_disa_minus20: "dISA −20",
    cruise_disa_plus10: "dISA +10",
    obstacle_preset_afm: "5 m / 16 ft (AFM)",
    obstacle_preset_screen: "15 m / 50 ft",
    strip_fig54_base: "Fig 5.4 · base",
    strip_fig54_wt: "Wt",
    strip_fig54_hw: "HW",
    strip_panel_oat_pa: "OAT / PA",
    strip_panel_weight: "Peso",
    strip_panel_hw: "HW kt",
    strip_panel_obstacle: "Obstáculo",
    strip_fig52_ea: "Fig 5.2 EA",
    strip_fig55_roc: "Fig 5.5 ROC",
    strip_fig56_roc: "Fig 5.6 ROC",
    strip_fig58_roc: "Fig 5.8 ROC",
    takeoff_ground_roll: "Rodaje",
    takeoff_over_obstacle: "Sobre obstáculo de",
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
    tab_readings: "Readings",
    tab_manual: "Manual",
    lang_toggle: "Language",
    theme_toggle: "Theme",
    theme_system: "System",
    theme_light: "Light",
    theme_dark: "Dark",
    subtitle: "AFM DA202-C1 Rev 29 (30 Jun 2021) — digitized Chapter 5",
    inputs_title: "Inputs",
    results_title: "Results",
    save_reading: "Save reading",
    export_csv: "Export CSV",
    export_json: "Export JSON",
    field_elev: "Field elevation (ft)",
    qnh: "QNH",
    qnh_dual: "QNH (hPa / inHg)",
    oat: "OAT (°C)",
    runway: "Runway",
    wind: "Wind (° / kt)",
    weight: "Weight (lb)",
    cruise_pa: "Cruise PA (ft)",
    cruise_rpm: "Cruise RPM",
    obstacle_m: "Obstacle height (m)",
    climb_figure: "Climb configuration (AFM)",
    climb_fig_55: "Fig 5.5 — cruise flaps",
    climb_fig_56: "Fig 5.6 — takeoff flaps",
    climb_fig_58: "Fig 5.8 — balked landing",
    climb_flaps_to: "T/O flaps",
    climb_flaps_up: "UP flaps",
    fuel_usable: "Usable fuel (US gal)",
    fuel_reserve: "Reserve (US gal)",
    derived_not_afm: "Derived, not an AFM chart",
    landing_no_corr: "No wind, temperature, or weight correction (Table 4).",
    takeoff: "Takeoff (Fig 5.4)",
    climb: "Climb (via Fig 5.2)",
    cruise: "Cruise (Table 3)",
    landing: "Landing (Table 4)",
    wind_card: "Wind (Fig 5.3)",
    unavailable: "Unavailable",
    derived_time: "Derived time (no AFM chart): ",
    plot_hint: "Drag the cursor on digitized curves (not AFM scans). Pointer and touch supported.",
    plot_readout_chart: "Chart read",
    plot_readout_core: "Core result",
    pa_override_chip: "Chart PA",
    pa_override_reset: "Reset PA",
    readings_compare: "Compare readings",
    compare_pick_a: "Reading A",
    compare_pick_b: "Reading B",
    axis_hw_kt: "HW kt",
    axis_oat_c: "OAT °C",
    axis_base_roll_m: "Base roll m",
    axis_ea_ft: "EA ft",
    axis_roc_fpm: "ROC fpm",
    axis_pa_ft: "PA ft",
    axis_dist_ft: "ft",
    axis_angle_deg: "Angle °",
    provenance_rev: " rev ",
    cruise_oat: "Cruise OAT (°C)",
    cruise_isa: "ISA",
    cruise_disa_minus20: "dISA −20",
    cruise_disa_plus10: "dISA +10",
    obstacle_preset_afm: "5 m / 16 ft (AFM)",
    obstacle_preset_screen: "15 m / 50 ft",
    strip_fig54_base: "Fig 5.4 · base",
    strip_fig54_wt: "Wt",
    strip_fig54_hw: "HW",
    strip_panel_oat_pa: "OAT / PA",
    strip_panel_weight: "Weight",
    strip_panel_hw: "HW kt",
    strip_panel_obstacle: "Obstacle",
    strip_fig52_ea: "Fig 5.2 EA",
    strip_fig55_roc: "Fig 5.5 ROC",
    strip_fig56_roc: "Fig 5.6 ROC",
    strip_fig58_roc: "Fig 5.8 ROC",
    takeoff_ground_roll: "Ground roll",
    takeoff_over_obstacle: "Over obstacle",
    manual_title: "Methods manual",
    manual_intro: "Each calculation path, AFM source, interpolation, and limitations.",
    afm_link: "Official AFM Rev 29",
    portal_link: "Diamond technical publications",
    amm_link: "AMM DA201-C1 Rev 25",
    methods: &EN_METHODS,
};

const ES_METHODS: [MethodCopy; 8] = [
    MethodCopy {
        title: "Altitud de presión y densidad",
        afm: "AFM DA202-C1 Rev 29 — entradas de rendimiento",
        summary: "PA (ft) = elevación + (1013,25 − QNH) × 30. La PA directa de carta anula elevación/QNH.",
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
        limitations: "Tras escala carta 1 ~3,9 %; error residual <0,5 % vs ejemplo Rev 29 (21 °C, 4 kt HW, 5 m).",
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
        summary: "Fila por PA y RPM; banda OAT dISA−20 / ISA / dISA+10.",
        interpolation: "Interpolación PA entre filas; banda por OAT−ISA.",
        limitations: "GPH transcritos tal como impreso (anomalías conocidas).",
    },
    MethodCopy {
        title: "Aterrizaje (Tabla 4)",
        afm: "AFM p. 5-17, Tabla 4",
        summary: "Consulta por altitud de presión únicamente (ft).",
        interpolation: "Lineal en altitudes de la tabla.",
        limitations: "Sin corrección por viento, temperatura ni peso. 1360 ft ≈ 415 m; texto AFM cita 414 m.",
    },
    MethodCopy {
        title: "Componentes de viento (Fig 5.3)",
        afm: "AFM p. 5-8, Figura 5.3",
        summary: "Viento de frente y cruzado por trigonometría; formato 200 / 08 kt.",
        interpolation: "N/A para componentes.",
        limitations: "Rumbo pista = número × 10°; aviso si pista no numérica.",
    },
    MethodCopy {
        title: "Autonomía y alcance (derivado)",
        afm: "AFM §1.8, §5.3.10 (duración máxima) + Tabla 3 GPH",
        summary: "(Combustible utilizable − reserva) / caudal Tabla 3; alcance = autonomía × TAS.",
        interpolation: "Usa crucero interpolado de Tabla 3.",
        limitations: "Derivado, no carta AFM. Combustible utilizable 24,0 US gal (total 24,5; AFM §1.8). Placas: 21,3 / 19,8 gal. Ver AFM §5.3.10 duración máxima.",
    },
];

const EN_METHODS: [MethodCopy; 8] = [
    MethodCopy {
        title: "Pressure & density altitude",
        afm: "AFM DA202-C1 Rev 29 — performance inputs",
        summary:
            "PA (ft) = elevation + (1013.25 − QNH) × 30. Direct chart PA overrides elevation/QNH.",
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
        limitations: "Sheet 1 OAT/PA scale ~3.9%; residual <0.5% vs Rev 29 example (21 °C, 4 kt HW, 5 m).",
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
        summary: "Row match on PA and RPM; OAT band dISA−20 / ISA / dISA+10.",
        interpolation: "PA interpolation between rows.",
        limitations: "GPH as printed (known oddities).",
    },
    MethodCopy {
        title: "Landing (Table 4)",
        afm: "AFM p. 5-17, Table 4",
        summary: "Pressure-altitude lookup only (ft).",
        interpolation: "Linear on table altitudes.",
        limitations:
            "No wind, temperature, or weight correction. 1360 ft ≈ 415 m; AFM text cites 414 m.",
    },
    MethodCopy {
        title: "Wind components (Fig 5.3)",
        afm: "AFM p. 5-8, Figure 5.3",
        summary: "Headwind/crosswind by trigonometry; display 200 / 08 kt.",
        interpolation: "N/A for components.",
        limitations: "Runway heading = number × 10°; warning if non-numeric runway.",
    },
    MethodCopy {
        title: "Endurance & range (derived)",
        afm: "AFM §1.8, §5.3.10 (Maximum Flight Duration) + Table 3 GPH",
        summary: "(Usable fuel − reserve) / Table 3 flow; range = endurance × TAS.",
        interpolation: "Uses interpolated Table 3 cruise.",
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

pub fn phase_slug_label(locale: Locale, slug: &str) -> String {
    let c = copy(locale);
    match slug {
        "takeoff" => c.tab_takeoff.to_string(),
        "climb" => c.tab_climb.to_string(),
        "cruise" => c.tab_cruise.to_string(),
        "endurance" => c.tab_endurance.to_string(),
        "landing" => c.tab_landing.to_string(),
        "readings" => c.tab_readings.to_string(),
        "manual" => c.tab_manual.to_string(),
        other => other.to_string(),
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

pub fn warn_text(locale: Locale, w: &da20_core::types::CalculationWarning) -> String {
    if !w.message.is_empty() {
        return w.message.clone();
    }
    let c = copy(locale);
    let ctx = w.args.first().map(String::as_str).unwrap_or("");
    let a1 = w.args.get(1).map(String::as_str).unwrap_or("");
    let a2 = w.args.get(2).map(String::as_str).unwrap_or("");
    match w.code.as_str() {
        "CLAMP_LOW" => format!("{ctx}: {a1} < min {a2} (clamp)"),
        "CLAMP_HIGH" => format!("{ctx}: {a1} > max {a2} (clamp)"),
        "EXTRAP_LOW" | "EXTRAP_HIGH" => format!("{ctx}: extrapolation ({a1})"),
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
        "DERIVED_ESTIMATE" => match locale {
            Locale::Es => "Estimación derivada de trepada (sin carta AFM).".into(),
            Locale::En => "Derived climb estimate (no AFM chart).".into(),
        },
        "TAKEOFF_OBSTACLE_BIAS" => match locale {
            Locale::Es => {
                "Fig 5.4: escala carta 1; residual <0,5 % vs ejemplo Rev 29 (21 °C).".into()
            }
            Locale::En => {
                "Fig 5.4: sheet 1 scale; residual <0.5% vs Rev 29 example (21 °C).".into()
            }
        },
        "UNITS" => match locale {
            Locale::Es => "Despegue: carta en m, pantalla en ft.".into(),
            Locale::En => "Takeoff: chart in m, display in ft.".into(),
        },
        "TABLE3_GPH" => match locale {
            Locale::Es => "GPH Tabla 3 transcritos del AFM Rev 29.".into(),
            Locale::En => "Table 3 GPH transcribed from AFM Rev 29.".into(),
        },
        "RUNWAY_HEADING" => match locale {
            Locale::Es => format!("Pista '{ctx}' sin rumbo numérico — 0°."),
            Locale::En => format!("Runway '{ctx}' has no numeric heading — 0°."),
        },
        "TAILWIND" => match locale {
            Locale::Es => format!("Cola {a1} kt."),
            Locale::En => format!("Tailwind {a1} kt."),
        },
        other => other.to_string(),
    }
}
