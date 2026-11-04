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
    qnh: "QNH (hPa)",
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
    qnh: "QNH (hPa)",
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
    manual_title: "Methods manual",
    manual_intro: "Each calculation path, AFM source, interpolation, and limitations.",
    afm_link: "Official AFM Rev 29",
    portal_link: "Diamond technical publications",
    amm_link: "AMM DA201-C1 Rev 25",
    methods: &EN_METHODS,
};

const ES_METHODS: [MethodCopy; 7] = [
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
        limitations: "Sesgo ~2,7 % bajo vs ejemplo Rev 29 sobre obstáculo de 5 m.",
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
];

const EN_METHODS: [MethodCopy; 7] = [
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
        limitations: "~2.7% short vs Rev 29 worked example over 5 m obstacle.",
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
];
