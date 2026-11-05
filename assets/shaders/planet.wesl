// Matériau des planètes à apparence peinte (M9, lot L6 ; Hexis, monde-usine, Q17).
//
// Étend le `StandardMaterial` du jeu (crates/sh_game/src/views/planet.rs) : sa couleur de base
// est la carte équirectangulaire de jour. Ce shader y ajoute, en une seule passe :
// - le détail carrelable (projection triplanaire à deux échelles), qui module la couleur selon
//   les données de surface (ville, industrie, relief) : net en orbite basse, effacé au loin par
//   les mipmaps ;
// - les lumières des villes, seulement côté nuit, allumées en douceur au terminateur ;
// - un voile de brume éclairé par l'étoile, épais au limbe (fresnel).
// Repère des cartes (contrat avec tools/planets/hexis.py) : lat = π/2 − π v, lon = 2π u,
// direction (cos lat cos lon, sin lat, cos lat sin lon).

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{view, lights},
};

struct PlanetParams {
    // x, y : carreaux par rayon des deux couches de détail ; z, w : leurs poids.
    detail_scale: vec4<f32>,
    // x, y : flou des deux couches (niveaux de mipmap) ; z : netteté du mélange triplanaire.
    detail_misc: vec4<f32>,
    // Force de la modulation : x ville, y industrie, z relief, w lumières.
    detail_strength: vec4<f32>,
    // rgb : teinte des lumières × intensité.
    night_color: vec4<f32>,
    // rgb : teinte des centres les plus éclairés × intensité.
    night_core: vec4<f32>,
    // x, y : allumage selon −N·L ; z, w : éclairage du voile selon N·L.
    terminator: vec4<f32>,
    // rgb : teinte du voile ; w : opacité au limbe.
    haze: vec4<f32>,
    // x : puissance du fresnel ; y : voile minimal.
    haze_shape: vec4<f32>,
    // Fini : x rugosité des terres, y de l'eau, z des usines ; w part de métal des usines.
    surface_finish: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> planet: PlanetParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var surface_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var surface_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var detail_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var detail_sampler: sampler;

const PI: f32 = 3.14159265358979;

// Direction du point de surface dans le repère des cartes (continue à la couture u = 0 / 1).
fn sphere_dir(uv: vec2<f32>) -> vec3<f32> {
    let lat = 0.5 * PI - PI * uv.y;
    let lon = 2.0 * PI * uv.x;
    return vec3<f32>(cos(lat) * cos(lon), sin(lat), cos(lat) * sin(lon));
}

// Projection triplanaire à gradients explicites : les projections de poids négligeable sont
// sautées (la plupart des pixels n'en lisent qu'une), ce qu'interdit un `textureSample` simple
// hors d'un flot de contrôle uniforme.
fn triplanar(p: vec3<f32>, w: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> vec4<f32> {
    var acc = vec4<f32>(0.0);
    if w.x > 0.0 {
        acc += w.x * textureSampleGrad(detail_texture, detail_sampler, p.yz, dx.yz, dy.yz);
    }
    if w.y > 0.0 {
        acc += w.y * textureSampleGrad(detail_texture, detail_sampler, p.zx, dx.zx, dy.zx);
    }
    if w.z > 0.0 {
        acc += w.z * textureSampleGrad(detail_texture, detail_sampler, p.xy, dx.xy, dy.xy);
    }
    return acc;
}

// Une couche de détail, divisée par la moyenne de la texture (moyenne 1 : aucun décalage).
fn detail_layer(
    d: vec3<f32>,
    w: vec3<f32>,
    ddx: vec3<f32>,
    ddy: vec3<f32>,
    scale: f32,
    blur: f32,
    mean: vec4<f32>,
) -> vec4<f32> {
    let g = scale * exp2(blur);
    return triplanar(d * scale, w, ddx * g, ddy * g) / mean;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Données de surface : R lumières, G ville, B industrie, A eau.
    let surface = textureSample(surface_texture, surface_sampler, in.uv);
    let urban = surface.g;
    let industry = surface.b;
    let water = surface.a;
    let rough = (1.0 - water) * saturate(1.0 - urban - industry);

    // Détail : poids triplanaires resserrés, puis normalisés.
    let d = sphere_dir(in.uv);
    let ddx = dpdx(d);
    let ddy = dpdy(d);
    var w = pow(abs(d), vec3<f32>(planet.detail_misc.z));
    w = w / (w.x + w.y + w.z);
    w = select(vec3<f32>(0.0), w, w > vec3<f32>(0.003));
    w = w / (w.x + w.y + w.z);
    // Moyenne de la texture de détail : son dernier niveau de mipmap (1 × 1).
    let last = f32(textureNumLevels(detail_texture) - 1u);
    let mean = max(textureSampleLevel(detail_texture, detail_sampler, vec2<f32>(0.5), last), vec4<f32>(0.05));
    let coarse = detail_layer(d, w, ddx, ddy, planet.detail_scale.x, planet.detail_misc.x, mean);
    let fine = detail_layer(d, w, ddx, ddy, planet.detail_scale.y, planet.detail_misc.y, mean);

    let s = planet.detail_strength;
    let wc = planet.detail_scale.z;
    let wf = planet.detail_scale.w;
    let modulation = wc * (urban * s.x * (coarse.r - 1.0) + industry * s.y * (coarse.b - 1.0) + rough * s.z * (coarse.a - 1.0))
        + wf * (urban * s.x * (fine.r - 1.0) + industry * s.y * (fine.b - 1.0) + rough * s.z * (fine.a - 1.0));
    let lights_detail = max(1.0 + s.w * wc * (coarse.g - 1.0), 0.0) * max(1.0 + s.w * wf * (fine.g - 1.0), 0.0);

    let base = pbr_input.material.base_color;
    pbr_input.material.base_color = vec4<f32>(base.rgb * max(1.0 + modulation, 0.0), base.a);
    // Usines : toits et cuves métalliques qui accrochent le soleil ; eau plus lisse encore.
    let finish = planet.surface_finish;
    pbr_input.material.perceptual_roughness = mix(mix(finish.x, finish.z, industry), finish.y, water);
    pbr_input.material.metallic = finish.w * industry * (1.0 - water);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

    // L'étoile : la seule lumière directionnelle de la vue Globale. Sans elle, tout est nuit.
    var to_sun = vec3<f32>(0.0, 1.0, 0.0);
    var sun = vec3<f32>(0.0);
    var has_sun = 0.0;
    if lights.n_directional_lights > 0u {
        to_sun = lights.directional_lights[0].direction_to_light;
        // Éclairement après exposition, ÷ π : la radiance d'un blanc parfait éclairé de face.
        sun = lights.directional_lights[0].color.rgb * view.exposure / PI;
        has_sun = 1.0;
    }
    let n = normalize(in.world_normal);
    let ndl = dot(n, to_sun);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    // Lumières des villes, côté nuit seulement. Ajoutées après l'éclairage : comme l'émissif,
    // elles ne dépendent pas de l'exposition (valeurs calibrées pour le jeu, AGENTS.md §4).
    let night = mix(1.0, smoothstep(planet.terminator.x, planet.terminator.y, -ndl), has_sun);
    let glow = surface.r * min(lights_detail, 3.0);
    let tint = mix(planet.night_color.rgb, planet.night_core.rgb, smoothstep(0.5, 1.2, glow));
    out.color = vec4<f32>(out.color.rgb + tint * glow * night, out.color.a);

    // Voile de brume : éclairé par l'étoile, épais au limbe, il voile aussi les lumières.
    let ndv = saturate(dot(n, pbr_input.V));
    let fresnel = pow(1.0 - ndv, planet.haze_shape.x);
    let veil = saturate((planet.haze_shape.y + (1.0 - planet.haze_shape.y) * fresnel) * planet.haze.w);
    let lit = smoothstep(planet.terminator.z, planet.terminator.w, ndl);
    let haze = planet.haze.rgb * sun * lit;
    out.color = vec4<f32>(mix(out.color.rgb, haze, veil), out.color.a);

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
