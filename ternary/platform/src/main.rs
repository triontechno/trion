use trion_platform::CapabilityReport;

fn main() {
    print!("{}", CapabilityReport::for_build_target().to_report_text());
}
