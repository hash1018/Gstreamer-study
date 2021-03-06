use std::sync::{Arc, Mutex};

use byte_slice_cast::*;

use glib::source::SourceId;
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use gst_audio::AudioInfo;

const CHUNK_SIZE: usize = 1024; // Amount of bytes we are sending in each buffer
const SAMPLE_RATE: u32 = 44_100; // Samples per second we are sending

#[derive(Debug)]
struct CustomData {
    source_id: Option<SourceId>,

    num_samples: u64, // Number of samples generated so far (for timestamp generation)
    // For waveform generation
    a: f64,
    b: f64,
    c: f64,
    d: f64,

    appsrc: AppSrc,
    appsink: AppSink,
}

impl CustomData {
    fn new(appsrc: &AppSrc, appsink: &AppSink) -> CustomData {
        CustomData {
            source_id: None,
            num_samples: 0,
            a: 0.0,
            b: 1.0,
            c: 0.0,
            d: 1.0,
            appsrc: appsrc.clone(),
            appsink: appsink.clone(),
        }
    }
}

fn main() {
    // Initialize GStreamer
    if let Err(err) = gst::init() {
        eprintln!("Failed to initialize Gst: {}", err);
        return;
    }

    let appsrc = gst::ElementFactory::make("appsrc", Some("audio_source")).unwrap();
    let tee = gst::ElementFactory::make("tee", Some("tee")).unwrap();
    let audio_queue = gst::ElementFactory::make("queue", Some("audio_queue")).unwrap();
    let audio_convert1 = gst::ElementFactory::make("audioconvert", Some("audio_convert1")).unwrap();
    let audio_resample =
        gst::ElementFactory::make("audioresample", Some("audio_resample")).unwrap();
    let audio_sink = gst::ElementFactory::make("autoaudiosink", Some("audio_sink")).unwrap();
    let video_queue = gst::ElementFactory::make("queue", Some("video_queue")).unwrap();
    let audio_convert2 = gst::ElementFactory::make("audioconvert", Some("audio_convert2")).unwrap();
    let visual = gst::ElementFactory::make("wavescope", Some("visual")).unwrap();
    let video_convert = gst::ElementFactory::make("videoconvert", Some("video_convert")).unwrap();
    let video_sink = gst::ElementFactory::make("autovideosink", Some("video_sink")).unwrap();
    let app_queue = gst::ElementFactory::make("queue", Some("app_queue")).unwrap();
    let appsink = gst::ElementFactory::make("appsink", Some("app_sink")).unwrap();

    let pipeline = gst::Pipeline::new(Some("test-pipeline"));

    visual.set_property_from_str("shader", "none");
    visual.set_property_from_str("style", "lines");

    pipeline
        .add_many(&[
            &appsrc,
            &tee,
            &audio_queue,
            &audio_convert1,
            &audio_resample,
            &audio_sink,
            &video_queue,
            &audio_convert2,
            &visual,
            &video_convert,
            &video_sink,
            &app_queue,
            &appsink,
        ])
        .unwrap();

    gst::Element::link_many(&[&appsrc, &tee]).unwrap();
    gst::Element::link_many(&[&audio_queue, &audio_convert1, &audio_resample, &audio_sink])
        .unwrap();
    gst::Element::link_many(&[
        &video_queue,
        &audio_convert2,
        &visual,
        &video_convert,
        &video_sink,
    ])
    .unwrap();
    gst::Element::link_many(&[&app_queue, &appsink]).unwrap();

    let tee_audio_pad = tee.request_pad_simple("src_%u").unwrap();
    println!(
        "Obtained request pad {} for audio branch",
        tee_audio_pad.name()
    );
    let queue_audio_pad = audio_queue.static_pad("sink").unwrap();
    tee_audio_pad.link(&queue_audio_pad).unwrap();

    let tee_video_pad = tee.request_pad_simple("src_%u").unwrap();
    println!(
        "Obtained request pad {} for video branch",
        tee_video_pad.name()
    );
    let queue_video_pad = video_queue.static_pad("sink").unwrap();
    tee_video_pad.link(&queue_video_pad).unwrap();
    let tee_app_pad = tee.request_pad_simple("src_%u").unwrap();
    let queue_app_pad = app_queue.static_pad("sink").unwrap();
    tee_app_pad.link(&queue_app_pad).unwrap();

    // configure appsrc

    // The first property that needs to be set on the appsrc is caps.
    // It specifies the kind of data that the element is going to produce,
    // so GStreamer can check if linking with downstream elements is possible
    // (this is, if the downstream elements will understand this kind of data).
    // This property must be a GstCaps object, which is easily built from a string with gst_caps_from_string().

    // We then connect to the need-data and enough-data signals.
    // These are fired by appsrc when its internal queue of data is running low or almost full, respectively.
    // We will use these signals to start and stop (respectively) our signal generation process.

    let info = AudioInfo::builder(gst_audio::AudioFormat::S16le, SAMPLE_RATE, 1)
        .build()
        .unwrap();
    let audio_caps = info.to_caps().unwrap();

    let appsrc = appsrc
        .dynamic_cast::<AppSrc>()
        .expect("Source element is expected to be an appsrc!");
    appsrc.set_caps(Some(&audio_caps));
    appsrc.set_format(gst::Format::Time);

    // Regarding the appsink configuration, we connect to the new-sample signal,
    // which is emitted every time the sink receives a buffer.
    // Also, the signal emission needs to be enabled through the emit-signals property, because, by default, it is disabled.

    let appsink = appsink
        .dynamic_cast::<AppSink>()
        .expect("Sink element is expected to be an appsink!");

    let data: Arc<Mutex<CustomData>> = Arc::new(Mutex::new(CustomData::new(&appsrc, &appsink)));

    let data_weak = Arc::downgrade(&data);
    let data_weak2 = Arc::downgrade(&data);
    appsrc.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |_, _size| {
                let data = match data_weak.upgrade() {
                    Some(data) => data,
                    None => return,
                };
                let mut d = data.lock().unwrap();

                if d.source_id.is_none() {
                    println!("start feeding");

                    let data_weak = Arc::downgrade(&data);
                    d.source_id = Some(glib::source::idle_add(move || {
                        let data = match data_weak.upgrade() {
                            Some(data) => data,
                            None => return glib::Continue(false),
                        };

                        let (appsrc, buffer) = {
                            let mut data = data.lock().unwrap();
                            let mut buffer = gst::Buffer::with_size(CHUNK_SIZE).unwrap();
                            let num_samples = CHUNK_SIZE / 2; /* Each sample is 16 bits */
                            let pts = gst::ClockTime::SECOND
                                .mul_div_floor(data.num_samples, u64::from(SAMPLE_RATE))
                                .expect("u64 overflow");
                            let duration = gst::ClockTime::SECOND
                                .mul_div_floor(num_samples as u64, u64::from(SAMPLE_RATE))
                                .expect("u64 overflow");

                            {
                                let buffer = buffer.get_mut().unwrap();
                                {
                                    let mut samples = buffer.map_writable().unwrap();
                                    let samples = samples.as_mut_slice_of::<i16>().unwrap();

                                    // Generate some psychodelic waveforms
                                    data.c += data.d;
                                    data.d -= data.c / 1000.0;
                                    let freq = 1100.0 + 1000.0 * data.d;

                                    for sample in samples.iter_mut() {
                                        data.a += data.b;
                                        data.b -= data.a / freq;
                                        *sample = 500 * (data.a as i16);
                                        //*sample = 0;
                                    }

                                    data.num_samples += num_samples as u64;
                                }

                                buffer.set_pts(pts);
                                buffer.set_duration(duration);
                            }

                            (data.appsrc.clone(), buffer)
                        };

                        glib::Continue(appsrc.push_buffer(buffer).is_ok())
                    }));
                }
            })
            .enough_data(move |_| {
                let data = match data_weak2.upgrade() {
                    Some(data) => data,
                    None => return,
                };

                let mut data = data.lock().unwrap();
                if let Some(_source) = data.source_id.take() {
                    println!("stop feeding");
                    //source.remove();
                }
            })
            .build(),
    );

    // configure appsink
    appsink.set_caps(Some(&audio_caps));

    let data_weak = Arc::downgrade(&data);
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |_| {
                let data = match data_weak.upgrade() {
                    Some(data) => data,
                    None => return Ok(gst::FlowSuccess::Ok),
                };

                let appsink = {
                    let data = data.lock().unwrap();
                    data.appsink.clone()
                };

                if let Ok(_sample) = appsink.pull_sample() {
                    use std::io::{self, Write};
                    // The only thing we do in this example is print a * to indicate a received buffer
                    print!("*");
                    let _ = io::stdout().flush();
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    let main_loop = glib::MainLoop::new(None, false);
    let main_loop_clone = main_loop.clone();
    let bus = pipeline.bus().unwrap();
    #[allow(clippy::single_match)]
    bus.connect_message(Some("error"), move |_, msg| match msg.view() {
        gst::MessageView::Error(err) => {
            let main_loop = &main_loop_clone;
            eprintln!(
                "Error received from element {:?}: {}",
                err.src().map(|s| s.path_string()),
                err.error()
            );
            eprintln!("Debugging information: {:?}", err.debug());
            main_loop.quit();
        }

        gst::MessageView::StateChanged(state_changed) => {
            println!("state changed {:?} ", state_changed.current());
        }
        _ => unreachable!(),
    });
    bus.add_signal_watch();

    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state.");

    main_loop.run();

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state.");

    bus.remove_signal_watch();
}
