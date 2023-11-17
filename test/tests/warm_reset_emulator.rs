use caliptra_builder::{
    firmware::{APP_WITH_UART, FMC_WITH_UART, ROM_WITH_UART},
    ImageOptions,
};
use caliptra_common::mailbox_api::CommandId;
use caliptra_hw_model::{mbox_write_fifo, BootParams, HwModel, InitParams, SecurityState};
use caliptra_hw_model_types::{DeviceLifecycle, Fuses};
use caliptra_test::swap_word_bytes_inplace;
use openssl::sha::sha384;
use zerocopy::AsBytes;

fn bytes_to_be_words_48(buf: &[u8; 48]) -> [u32; 12] {
    let mut result: [u32; 12] = zerocopy::transmute!(*buf);
    swap_word_bytes_inplace(&mut result);
    result
}

#[test]
fn test_warm_reset_verilator() {
    let mut rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let init_params = InitParams {
        rom: &rom,
        ..Default::default()
    };
    let image_options = Default::default();

    let image =
        caliptra_builder::build_and_sign_image(&FMC_WITH_UART, &APP_WITH_UART, image_options)
            .unwrap();

    let vendor_pk_hash =
        bytes_to_be_words_48(&sha384(image.manifest.preamble.vendor_pub_keys.as_bytes()));
    let owner_pk_hash =
        bytes_to_be_words_48(&sha384(image.manifest.preamble.owner_pub_keys.as_bytes()));
    let mut hw_model = caliptra_hw_model::new(BootParams {
        init_params: init_params,
        fw_image: Some(&image.to_bytes().unwrap()),
        valid_pauser: 0x1,
        ..Default::default()
    })
    .unwrap();
    hw_model.step_until(|m| m.soc_ifc().cptra_flow_status().read().ready_for_fw());
    //    hw_model.step_until(|m|m.soc_mbox().status().read().mbox_fsm_ps().mbox_idle());
    hw_model
        .step_until_output_contains("Caliptra RT listening for mailbox commands...")
        .unwrap();
    hw_model.warm_reset_flow(&Fuses {
        key_manifest_pk_hash: vendor_pk_hash,
        owner_pk_hash,
        fmc_key_manifest_svn: 0b11111111,
        ..Default::default()
    });
    hw_model
        .step_until_output_contains("Caliptra RT listening for mailbox commands...")
        .unwrap();
}

#[test]
fn test_warm_reset_em() {
    let security_state = *SecurityState::default()
        .set_debug_locked(true)
        .set_device_lifecycle(DeviceLifecycle::Production);

    let rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let image = caliptra_builder::build_and_sign_image(
        &FMC_WITH_UART,
        &APP_WITH_UART,
        ImageOptions {
            fmc_min_svn: 5,
            fmc_svn: 9,
            ..Default::default()
        },
    )
    .unwrap();
    let vendor_pk_hash =
        bytes_to_be_words_48(&sha384(image.manifest.preamble.vendor_pub_keys.as_bytes()));
    let owner_pk_hash =
        bytes_to_be_words_48(&sha384(image.manifest.preamble.owner_pub_keys.as_bytes()));

    let mut hw = caliptra_hw_model::new(BootParams {
        init_params: InitParams {
            rom: &rom,
            security_state,
            ..Default::default()
        },
        fuses: Fuses {
            key_manifest_pk_hash: vendor_pk_hash,
            owner_pk_hash,
            fmc_key_manifest_svn: 0b1111111,
            ..Default::default()
        },
        fw_image: Some(&image.to_bytes().unwrap()),
        ..Default::default()
    })
    .unwrap();

    // Wait for boot
    while !hw.soc_ifc().cptra_flow_status().read().ready_for_runtime() {
        hw.step();
    }
    
    // Perform warm reset
    hw.warm_reset_flow(&Fuses {
        key_manifest_pk_hash: vendor_pk_hash,
        owner_pk_hash,
        fmc_key_manifest_svn: 0b1111111,
        ..Default::default()
    });

    // Wait for boot
    while !hw.soc_ifc().cptra_flow_status().read().ready_for_runtime() {
        hw.step();
    }
}
