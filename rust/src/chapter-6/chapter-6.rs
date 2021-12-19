use gst::prelude::*;

#[path = "../common.rs"]
mod common;

fn print_caps(caps: &gst::Caps, prefix: &str) {
    println!("===== print_caps =====");
    if caps.is_any() {
        println!("{}ANY", prefix);
        return;
    }

    if caps.is_empty() {
        println!("{}EMPTY", prefix);
        return;
    }

    for structure in caps.iter() {
        println!("{}{}", prefix, structure.name());
        for (field, value) in structure.iter() {
            println!(
                "{}  {}:{}",
                prefix,
                field,
                value.serialize().unwrap().as_str()
            );
        }
    }
}

// Prints information about a Pad Template, including its Capabilitites

// Pads allow information to enter and leave an element.
// The Capabilities (or Caps, for short) of a Pad, then, specify what kind of information can travel through the Pad.
// For example, “RGB video with a resolution of 320x200 pixels and 30 frames per second”, or “16-bits per sample audio,
// 5.1 channels at 44100 samples per second”, or even compressed formats like mp3 or h264.
// Pads can support multiple Capabilitie (for example, a video sink can support video in different types of RGB or YUV formats)
// and Capabilities can be specified as ranges (for example, an audio sink can support samples rates from 1 to 48000 samples per second).
// However, the actual information traveling from Pad to Pad must have only one well-specified type.
// Through a process known as negotiation, two linked Pads agree on a common type,
// and thus the Capabilities of the Pads become fixed (they only have one type and do not contain ranges).
// The walkthrough of the sample code below should make all this clear.

fn print_pad_template_information(factory: &gst::ElementFactory) {
    println!("===== print_pad_template_information =====");
    let long_name = factory
        .metadata("long-name")
        .expect("Failed to get long-name of element factory.");
    println!("Pad Template for {}:", long_name);

    if factory.num_pad_templates() == 0u32 {
        println!("  None");
        return;
    }

    // Pads are created from Pad Templates, which indicate all possible Capabilities a Pad could ever have.
    // Templates are useful to create several similar Pads, and also allow early refusal of connections between elements: 
    // If the Capabilities of their Pad Templates do not have a common subset (their intersection is empty), there is no need to negotiate further.
    // Pad Templates can be viewed as the first step in the negotiation process. 
    // As the process evolves, actual Pads are instantiated and their Capabilities refined until they are fixed (or negotiation fails).

    for pad_template in factory.static_pad_templates() {
        if pad_template.direction() == gst::PadDirection::Src {
            println!("  SRC template: '{}'", pad_template.name_template());
        } else if pad_template.direction() == gst::PadDirection::Sink {
            println!("  SINK template: '{}'", pad_template.name_template());
        } else {
            println!("  UNKNOWN!!! template: '{}'", pad_template.name_template());
        }

        if pad_template.presence() == gst::PadPresence::Always {
            println!("  Availability: Always");
        } else if pad_template.presence() == gst::PadPresence::Sometimes {
            println!("  Availability: Sometimes");
        } else if pad_template.presence() == gst::PadPresence::Request {
            println!("  Availability: On request");
        } else {
            println!("  Availability: UNKNOWN!!!");
        }

        let caps = pad_template.caps();
        println!("  Capabilities:");
        print_caps(&caps, "    ");
    }
}

fn print_pad_capabilities(element: &gst::Element, pad_name: &str) {
    println!("===== print_pad_capabilities =====");
    let pad = element
        .static_pad(pad_name)
        .expect("Could not retrieve pad");

    println!("Caps for the {} pad:", pad_name);
    let caps = pad.current_caps().unwrap_or_else(|| pad.query_caps(None));
    print_caps(&caps, "      ");
}

//https://gstreamer.freedesktop.org/documentation/tutorials/basic/media-formats-and-pad-capabilities.html?gi-language=c
fn tutorial_main() {
    // Initialize GStreamer
    gst::init().unwrap();

    // Create the element factories
    let source_factory =
        gst::ElementFactory::find("audiotestsrc").expect("Failed to create audiotestsrc factory.");
    let sink_factory = gst::ElementFactory::find("autoaudiosink")
        .expect("Failed to create autoaudiosink factory.");

    // Print information about the pad templates of these factories
    print_pad_template_information(&source_factory);
    print_pad_template_information(&sink_factory);

    // Ask the factories to instantiate actual elements
    let source = source_factory
        .create(Some("source"))
        .expect("Failed to create source element");
    let sink = sink_factory
        .create(Some("sink"))
        .expect("Failed to create sink element");

    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("test-pipeline"));

    pipeline.add_many(&[&source, &sink]).unwrap();
    source.link(&sink).expect("Elements could not be linked.");

    // Print initial negotiated caps (in NULL state)
    println!("In NULL state:");
    print_pad_capabilities(&sink, "sink");

    // Start playing
    let res = pipeline.set_state(gst::State::Playing);
    if res.is_err() {
        eprintln!(
            "Unable to set the pipeline to the `Playing` state (check the bus for error messages)."
        )
    }

    // Wait until error, EOS or State Change
    let bus = pipeline.bus().unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => {
                println!(
                    "Error received from element {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            MessageView::Eos(..) => {
                println!("End-Of-Stream reached.");
                break;
            }
            MessageView::StateChanged(state_changed) =>
            // We are only interested in state-changed messages from the pipeline
            {
                if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                    let new_state = state_changed.current();
                    let old_state = state_changed.old();

                    println!(
                        "Pipeline state changed from {:?} to {:?}",
                        old_state, new_state
                    );
                    print_pad_capabilities(&sink, "sink");
                }
            }
            _ => (),
        }
    }

    // Shutdown pipeline
    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

fn main() {
    // tutorials_common::run is only required to set up the application environment on macOS
    // (but not necessary in normal Cocoa applications where this is set up automatically)
    common::run(tutorial_main);
}