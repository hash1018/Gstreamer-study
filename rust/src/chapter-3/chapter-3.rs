use gst::prelude::*;

#[path = "../common.rs"]
mod common;

//https://gstreamer.freedesktop.org/documentation/tutorials/basic/dynamic-pipelines.html?gi-language=c
#[allow(dead_code)]
fn tutorial_main() {
    // Initialize GStreamer
    gst::init().unwrap();

    // The ports through which GStreamer elements communicate with each other are called pads (GstPad).
    // There exists sink pads, through which data enters an element, and source pads, through which data exits an element.
    // It follows naturally that source elements only contain source pads, sink elements only contain sink pads, and filter elements contain both.

    // Create the elements //source element only contains src pad, through which data exists an element.

    // uridecodebin will internally instantiate all the necessary elements (sources, demuxers and decoders)
    // to turn a URI into raw audio and/or video streams. It does half the work that playbin does.
    // Since it contains demuxers, its source pads are not initially available and we will need to link to them on the fly.
    let source = gst::ElementFactory::make("uridecodebin", Some("source"))
        .expect("Could not create uridecodebin element.");

    // audioconvert is useful for converting between different audio formats,
    // making sure that this example will work on any platform,
    // since the format produced by the audio decoder might not be the same that the audio sink expects.
    let convert = gst::ElementFactory::make("audioconvert", Some("convert"))
        .expect("Could not create convert element.");

    // audioresample is useful for converting between different audio sample rates,
    // similarly making sure that this example will work on any platform,
    // since the audio sample rate produced by the audio decoder might not be one that the audio sink supports.
    let resample = gst::ElementFactory::make("audioresample", Some("resample"))
        .expect("Could not create resample element.");

    //sink element only contains sink pad, through which data enters an element.

    // The autoaudiosink is the equivalent of autovideosink seen in the previous tutorial,
    // for audio. It will render the audio stream to the audio card.
    let sink = gst::ElementFactory::make("autoaudiosink", Some("sink"))
        .expect("Could not create sink element.");

    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("test-pipeline"));

    // Build the pipeline Note that we are NOT linking the source at this
    // point. We will do it later.
    pipeline
        .add_many(&[&source, &convert, &resample, &sink])
        .unwrap();
    gst::Element::link_many(&[&convert, &resample, &sink]).expect("Elements could not be linked.");

    // Set the URI to play
    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";
    source.set_property("uri", uri).unwrap();

    //The main complexity when dealing with demuxers is that they cannot produce any information
    //until they have received some data and have had a chance to look at the container to see what is inside.
    //This is, demuxers start with no source pads to which other elements can link, and thus the pipeline must necessarily terminate at them.
    //The solution is to build the pipeline from the source down to the demuxer, and set it to run (play).
    //When the demuxer has received enough information to know about the number and kind of streams in the container,
    //it will start creating source pads. This is the right time for us to finish building the pipeline and attach it to the newly added demuxer pads.

    // Connect the pad-added signal
    source.connect_pad_added(move |src, src_pad| {
        println!("Received new pad {} from {}", src_pad.name(), src.name());

        let sink_pad = convert
            .static_pad("sink")
            .expect("Failed to get static sink pad from convert");
        if sink_pad.is_linked() {
            println!("We are already linked. Ignoring.");
            return;
        }

        let new_pad_caps = src_pad
            .current_caps()
            .expect("Failed to get caps of new pad.");
        let new_pad_struct = new_pad_caps
            .structure(0)
            .expect("Failed to get first structure of caps.");
        let new_pad_type = new_pad_struct.name();

        let is_audio = new_pad_type.starts_with("audio/x-raw");
        if !is_audio {
            println!(
                "It has type {} which is not raw audio. Ignoring.",
                new_pad_type
            );
            return;
        }

        let res = src_pad.link(&sink_pad);
        if res.is_err() {
            println!("Type is {} but link failed.", new_pad_type);
        } else {
            println!("Link succeeded (type {}).", new_pad_type);
        }
    });

    // Start playing
    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    // Wait until error or EOS
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => {
                eprintln!(
                    "Error received from element {:?} {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                eprintln!("Debugging information: {:?}", err.debug());
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                    println!(
                        "Pipeline state changed from {:?} to {:?}",
                        state_changed.old(),
                        state_changed.current()
                    );
                }
            }
            MessageView::Eos(..) => break,
            _ => (),
        }
    }

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

//video 
#[allow(dead_code)]
fn exercise() {
    // Initialize GStreamer
    gst::init().unwrap();

    let source = gst::ElementFactory::make("uridecodebin", Some("source"))
        .expect("Could not create uridecodebin element.");
    let convert = gst::ElementFactory::make("videoconvert", Some("convert"))
        .expect("Could not create convert element.");
    let scale = gst::ElementFactory::make("videoscale", Some("scale"))
        .expect("Could not create resample element.");
    let sink = gst::ElementFactory::make("autovideosink", Some("sink"))
        .expect("Could not create sink element.");

    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("test-pipeline"));

    // Build the pipeline Note that we are NOT linking the source at this
    // point. We will do it later.
    pipeline
        .add_many(&[&source, &convert, &scale, &sink])
        .unwrap();
    gst::Element::link_many(&[&convert, &scale, &sink]).expect("Elements could not be linked.");

    // Set the URI to play
    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";
    source.set_property("uri", uri).unwrap();

    //The main complexity when dealing with demuxers is that they cannot produce any information
    //until they have received some data and have had a chance to look at the container to see what is inside.
    //This is, demuxers start with no source pads to which other elements can link, and thus the pipeline must necessarily terminate at them.
    //The solution is to build the pipeline from the source down to the demuxer, and set it to run (play).
    //When the demuxer has received enough information to know about the number and kind of streams in the container,
    //it will start creating source pads. This is the right time for us to finish building the pipeline and attach it to the newly added demuxer pads.

    // Connect the pad-added signal
    source.connect_pad_added(move |src, src_pad| {
        println!("Received new pad {} from {}", src_pad.name(), src.name());

        let sink_pad = convert
            .static_pad("sink")
            .expect("Failed to get static sink pad from convert");
        if sink_pad.is_linked() {
            println!("We are already linked. Ignoring.");
            return;
        }

        let new_pad_caps = src_pad
            .current_caps()
            .expect("Failed to get caps of new pad.");
        let new_pad_struct = new_pad_caps
            .structure(0)
            .expect("Failed to get first structure of caps.");
        let new_pad_type = new_pad_struct.name();

        let is_video = new_pad_type.starts_with("video/x-raw");
        if !is_video {
            println!(
                "It has type {} which is not raw video. Ignoring.",
                new_pad_type
            );
            return;
        }

        let res = src_pad.link(&sink_pad);
        if res.is_err() {
            println!("Type is {} but link failed.", new_pad_type);
        } else {
            println!("Link succeeded (type {}).", new_pad_type);
        }
    });

    // Start playing
    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    // Wait until error or EOS
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => {
                eprintln!(
                    "Error received from element {:?} {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                eprintln!("Debugging information: {:?}", err.debug());
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                    println!(
                        "Pipeline state changed from {:?} to {:?}",
                        state_changed.old(),
                        state_changed.current()
                    );
                }
            }
            MessageView::Eos(..) => break,
            _ => (),
        }
    }

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");

    println!("pipeline NULL");
}

fn main() {
    // tutorials_common::run is only required to set up the application environment on macOS
    // (but not necessary in normal Cocoa applications where this is set up automatically)
    common::run(tutorial_main);
    //common::run(exercise);
}
