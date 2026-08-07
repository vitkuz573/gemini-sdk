use criterion::{criterion_group, criterion_main, Criterion};
use gemini_sdk::chat::{GenerationConfig, PreparedRequest};
use gemini_sdk::models::ModelCategory;
use gemini_sdk::proto::slots::build_inner_req_list;

fn bench_build_inner_req_list(c: &mut Criterion) {
    let prepared = PreparedRequest {
        prompt: "Explain quantum computing in simple terms.".to_string(),
        inline_images: vec![],
        config: Some(GenerationConfig {
            max_output_tokens: Some(1024),
            temperature: Some(0.7),
            ..Default::default()
        }),
        category: ModelCategory::Pro,
    };

    c.bench_function("build_inner_req_list", |b| {
        b.iter(|| {
            build_inner_req_list(&prepared, None, None, &[], "REQUEST-UUID", "en", None, "nonce");
        });
    });
}

criterion_group!(benches, bench_build_inner_req_list);
criterion_main!(benches);
