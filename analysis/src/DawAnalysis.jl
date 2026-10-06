"""
DAW spectral / loudness analysis. Invoked by Rust via JSON on stdin/stdout.

Never call this from an audio callback. The daemon owns process isolation.
"""
module DawAnalysis

using JSON3
using Statistics

export analyze_spectrum, analyze_loudness, handle_request

const BAND_COUNT = 24

function analyze_spectrum(samples::Vector{Float64}; sample_rate::Float64 = 48000.0)
    n = length(samples)
    if n == 0
        return Dict("bands_hz" => Float64[], "magnitudes_db" => Float64[])
    end
    windowed = samples .* hann(n)
    spec = abs.(fft_real(windowed))
    nyquist = sample_rate / 2
    bands = 10 .^ range(log10(20.0), log10(min(nyquist, 20000.0)); length = BAND_COUNT)
    mags = Float64[]
    freqs = (0:n-1) .* (sample_rate / n)
    for i in 1:length(bands)
        lo = i == 1 ? 0.0 : bands[i]
        hi = i == length(bands) ? nyquist : bands[min(i + 1, length(bands))]
        idx = findall(f -> lo <= f < hi, freqs)
        energy = isempty(idx) ? 1e-12 : mean(spec[idx] .^ 2)
        push!(mags, 10 * log10(energy + 1e-12))
    end
    return Dict("bands_hz" => collect(bands), "magnitudes_db" => mags)
end

function analyze_loudness(samples::Vector{Float64}; sample_rate::Float64 = 48000.0)
    if isempty(samples)
        return Dict("lufs_integrated" => -70.0, "true_peak_dbtp" => -70.0, "lra" => 0.0)
    end
    rms = sqrt(mean(samples .^ 2) + 1e-12)
    lufs = 20 * log10(rms) - 0.691
    peak = maximum(abs.(samples))
    true_peak = 20 * log10(peak + 1e-12)
    # Simplified LRA: 95th - 10th percentile of short-term RMS blocks
    block = max(1, Int(round(sample_rate * 0.4)))
    shorts = Float64[]
    i = 1
    while i + block - 1 <= length(samples)
        r = sqrt(mean(samples[i:i+block-1] .^ 2) + 1e-12)
        push!(shorts, 20 * log10(r))
        i += block ÷ 2
    end
    lra = isempty(shorts) ? 0.0 : (quantile(shorts, 0.95) - quantile(shorts, 0.10))
    return Dict("lufs_integrated" => lufs, "true_peak_dbtp" => true_peak, "lra" => lra)
end

function hann(n::Int)
    return [0.5 - 0.5 * cos(2π * (i - 1) / (n - 1)) for i in 1:n]
end

# Minimal radix-2-ish DFT for modest buffers (offline analysis only).
function fft_real(x::Vector{Float64})
    n = length(x)
    [sum(x[k] * cis(-2π * (j - 1) * (k - 1) / n) for k in 1:n) for j in 1:n]
end

function handle_request(req::Dict)
    op = String(get(req, "op", ""))
    samples = Float64.(get(req, "samples", Float64[]))
    sr = Float64(get(req, "sample_rate", 48000.0))
    if op == "spectrum"
        return analyze_spectrum(samples; sample_rate = sr)
    elseif op == "loudness"
        return analyze_loudness(samples; sample_rate = sr)
    else
        error("unknown op: $op")
    end
end

function main()
    line = readline(stdin)
    req = JSON3.read(line, Dict{String, Any})
    result = handle_request(req)
    println(JSON3.write(result))
end

if abspath(PROGRAM_FILE) == @__FILE__
    main()
end

end # module
