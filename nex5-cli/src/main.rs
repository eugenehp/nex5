use std::path::PathBuf;

use clap::{Parser, Subcommand};
use nex5_analyze::{analyze_file, FileAnalysisOptions};
use nex5_nwb::{read_nwb_file, write_nwb_file, NwbReadOptions, NwbWriteOptions};
use nex5_sort::{phy_to_file_data, KilosortPipeline, KilosortPipelineOptions, PhyImportOptions};
use nex5file::{
    export_spikes_to_file, read_nex5_file, write_nex5_file, SpikeExportFormat, SpikeExportOptions,
};

#[derive(Parser)]
#[command(name = "nex5", about = "NeuroExplorer .nex/.nex5 toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print session summary (variable counts and duration).
    Info {
        path: PathBuf,
    },
    /// Export spike/event timestamps to CSV or text.
    ExportSpikes {
        path: PathBuf,
        variable: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Convert .nex5 to NWB 2.x.
    ToNwb {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Convert NWB 2.x to .nex5.
    FromNwb {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Print PSTH summary for a neuron aligned to an event.
    Psth {
        path: PathBuf,
        neuron: String,
        event: String,
    },
    /// Run Kilosort-style sorting on a continuous variable.
    Sort {
        path: PathBuf,
        continuous: String,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 30_000.0)]
        sampling_rate: f64,
        #[arg(long, default_value_t = 4.0)]
        threshold: f64,
        #[arg(long, default_value_t = 1.0)]
        refractory_ms: f64,
        #[arg(long)]
        phy_dir: Option<PathBuf>,
    },
    /// Import Kilosort/Phy spike_times.npy + spike_clusters.npy into .nex5.
    ImportPhy {
        dir: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 30_000.0)]
        sampling_rate: f64,
        #[arg(long, default_value_t = true)]
        skip_noise: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Info { path } => cmd_info(&path)?,
        Commands::ExportSpikes {
            path,
            variable,
            output,
        } => cmd_export_spikes(&path, &variable, &output)?,
        Commands::ToNwb { input, output } => cmd_to_nwb(&input, &output)?,
        Commands::FromNwb { input, output } => cmd_from_nwb(&input, &output)?,
        Commands::Psth {
            path,
            neuron,
            event,
        } => cmd_psth(&path, &neuron, &event)?,
        Commands::Sort {
            path,
            continuous,
            output,
            sampling_rate,
            threshold,
            refractory_ms,
            phy_dir,
        } => cmd_sort(
            &path,
            &continuous,
            &output,
            sampling_rate,
            threshold,
            refractory_ms,
            phy_dir.as_ref(),
        )?,
        Commands::ImportPhy {
            dir,
            output,
            sampling_rate,
            skip_noise,
        } => cmd_import_phy(&dir, &output, sampling_rate, skip_noise)?,
    }
    Ok(())
}

fn cmd_info(path: &PathBuf) -> nex5file::Result<()> {
    let data = read_nex5_file(path)?;
    println!("comment: {}", data.comment);
    println!("timestamp_frequency_hz: {}", data.timestamp_frequency_hz);
    println!("beg_seconds: {}", data.beg_seconds);
    println!("end_seconds: {}", data.end_seconds);
    println!("neurons: {}", data.neuron_names().len());
    println!("events: {}", data.event_names().len());
    println!("continuous: {}", data.continuous_names().len());
    println!("intervals: {}", data.interval_names().len());
    Ok(())
}

fn cmd_export_spikes(path: &PathBuf, variable: &str, output: &PathBuf) -> nex5file::Result<()> {
    let data = read_nex5_file(path)?;
    let mut opts = SpikeExportOptions::new();
    opts.format = SpikeExportFormat::Csv;
    export_spikes_to_file(&data, variable, output, &opts)
}

fn cmd_to_nwb(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let data = read_nex5_file(input)?;
    write_nwb_file(&data, output, &NwbWriteOptions::default())?;
    Ok(())
}

fn cmd_from_nwb(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let data = read_nwb_file(input, &NwbReadOptions::default())?;
    write_nex5_file(&data, output)?;
    Ok(())
}

fn cmd_psth(path: &PathBuf, neuron: &str, event: &str) -> nex5file::Result<()> {
    let data = read_nex5_file(path)?;
    let result = analyze_file(&data, neuron, event, &FileAnalysisOptions::default())?;
    println!("events: {}", result.psth.n_events);
    println!("bins: {}", result.psth.counts.len());
    println!(
        "total_spikes_in_window: {}",
        result.psth.counts.iter().sum::<u64>()
    );
    Ok(())
}

fn cmd_sort(
    path: &PathBuf,
    continuous: &str,
    output: &PathBuf,
    sampling_rate: f64,
    threshold: f64,
    refractory_ms: f64,
    phy_dir: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = read_nex5_file(path)?;
    let pipeline = KilosortPipeline::new(KilosortPipelineOptions {
        detect_threshold: threshold,
        refractory_seconds: refractory_ms / 1000.0,
        ..Default::default()
    });
    let result = pipeline.sort_continuous(&data, continuous)?;
    if let Some(dir) = phy_dir {
        result.write_phy_folder(dir)?;
    }
    let sorted = pipeline.to_file_data(&result, sampling_rate, "sorted")?;
    write_nex5_file(&sorted, output)?;
    println!(
        "units: {}, spikes: {}, templates: {}",
        sorted.neuron_names().len(),
        result.spike_times.len(),
        result.n_units()
    );
    Ok(())
}

fn cmd_import_phy(
    dir: &PathBuf,
    output: &PathBuf,
    sampling_rate: f64,
    skip_noise: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let opts = PhyImportOptions {
        sampling_rate,
        timestamp_frequency_hz: sampling_rate,
        skip_noise_cluster: skip_noise,
        ..Default::default()
    };
    let data = phy_to_file_data(dir, &opts)?;
    write_nex5_file(&data, output)?;
    println!("units: {}", data.neuron_names().len());
    Ok(())
}
