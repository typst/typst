use pkcs8::DecodePrivateKey;
use sha2::{
    digest::const_oid::db::rfc5912::{ID_SHA_512, SHA_512_WITH_RSA_ENCRYPTION},
    Digest,
};
use std::ops::Range;

use cms::{
    cert::{
        x509::{
            der::{
                asn1::{OctetString, SetOfVec},
                oid::db::rfc5911::{ID_DATA, ID_SIGNED_DATA},
                Any, AnyRef, Encode,
            },
            spki::AlgorithmIdentifier,
            Certificate,
        },
        CertificateChoices, IssuerAndSerialNumber,
    },
    content_info::{CmsVersion, ContentInfo},
    signed_data::{
        CertificateSet, EncapsulatedContentInfo, SignedData, SignerIdentifier,
        SignerInfo, SignerInfos,
    },
};
use pdf_writer::{
    types::{FieldType, SigFlags},
    writers::{Field, Form},
    Finish, Name, Pdf, Primitive, Ref, Str,
};
use rsa::{traits::SignatureScheme, Pkcs1v15Sign, RsaPrivateKey};
use sha2::Sha512;

const SIG_SIZE: usize = 1024 * 4;

pub fn prepare(alloc: &mut Ref, pdf: &mut Pdf) -> (Range<usize>, Ref) {
    let form_ref = alloc.bump();
    let signature_field_ref = alloc.bump();

    let mut signature_field: Field = pdf.indirect(signature_field_ref).start();
    signature_field.field_type(FieldType::Signature);
    let mut signature_dict = signature_field.insert(Name(b"V")).dict();
    signature_dict.pair(Name(b"Type"), Name(b"Sig"));
    signature_dict.pair(Name(b"Filter"), Name(b"Adobe.PPKLite"));
    signature_dict.pair(Name(b"SubFilter"), Name(b"adbe.pkcs7.detached"));
    let mut placeholder = [0; SIG_SIZE];
    placeholder[0] = 255; // Make sure pdf-writer writes this array as binary
    let sig_end = signature_dict
        .pair(Name(b"Contents"), Str(&placeholder))
        .current_len();
    let sig_start = sig_end
        - SIG_SIZE * 2 // 2 chars to write each byte
        - 2; // take < and > into account;
    signature_dict
        .insert(Name(b"ByteRange"))
        .array()
        .items([0, sig_start as i32, sig_end as i32])
        .item(Str(b"typst-document-size"));
    signature_dict.finish();
    signature_field.finish();

    let mut form: Form = pdf.indirect(form_ref).start();
    form.fields([signature_field_ref]);
    form.sig_flags(SigFlags::SIGNATURES_EXIST);

    (sig_start..sig_end, form_ref)
}

pub fn write(range: Range<usize>, mut bytes: Vec<u8>) -> Vec<u8> {
    let needle = b"(typst-document-size)";
    let doc_size_start = bytes[range.end..]
        .windows(needle.len())
        .position(|x| x == needle)
        .unwrap();
    let doc_size_range = doc_size_start..(doc_size_start + needle.len());
    dbg!(&range, &doc_size_range);
    let mut actual_size = Vec::new();
    <i32 as pdf_writer::Primitive>::write(bytes.len() as i32, &mut actual_size);
    actual_size.extend(std::iter::repeat(b' ').take(needle.len() - actual_size.len()));
    bytes.splice(
        doc_size_range.start + range.end..doc_size_range.end + range.end,
        actual_size,
    );

    let mut hasher = Sha512::new();
    hasher.update(&bytes[0..range.start]);
    hasher.update(&bytes[range.end..]);
    let hashed = hasher.finalize();

    let priv_key =
        RsaPrivateKey::from_pkcs8_encrypted_pem(include_str!("../../../key.pem"), "abcd")
            .unwrap();
    let signer = Pkcs1v15Sign::new::<Sha512>();
    let signature =
        signer.sign(Some(&mut rand::rngs::OsRng), &priv_key, &hashed).unwrap();

    let pem_chain =
        Certificate::load_pem_chain(include_bytes!("../../../cert.pem")).unwrap();
    let sig_data = ContentInfo {
        content_type: ID_SIGNED_DATA,
        content: Any::from(
            AnyRef::try_from(
                SignedData {
                    version: CmsVersion::V0,
                    digest_algorithms: SetOfVec::try_from(vec![AlgorithmIdentifier {
                        oid: SHA_512_WITH_RSA_ENCRYPTION,
                        parameters: None,
                    }])
                    .unwrap(),
                    encap_content_info: EncapsulatedContentInfo {
                        econtent_type: ID_DATA,
                        econtent: None,
                    },
                    certificates: Some(CertificateSet(
                        SetOfVec::from_iter(
                            pem_chain
                                .clone()
                                .into_iter()
                                .map(CertificateChoices::Certificate),
                        )
                        .unwrap(),
                    )),
                    crls: None,
                    signer_infos: SignerInfos(
                        SetOfVec::from_iter(pem_chain.into_iter().map(|cert| {
                            SignerInfo {
                                version: CmsVersion::V1,
                                sid: SignerIdentifier::IssuerAndSerialNumber(
                                    IssuerAndSerialNumber {
                                        issuer: cert.tbs_certificate.issuer,
                                        serial_number: cert.tbs_certificate.serial_number,
                                    },
                                ),
                                digest_alg: AlgorithmIdentifier {
                                    oid: ID_SHA_512,
                                    parameters: None,
                                },
                                signed_attrs: None, // TODO: should contain revocation information (see section 12.8.3.3.2)
                                signature_algorithm: AlgorithmIdentifier {
                                    oid: SHA_512_WITH_RSA_ENCRYPTION,
                                    parameters: None,
                                },
                                signature: OctetString::new(&signature[..]).unwrap(),
                                unsigned_attrs: None, // TODO: should contain timestamp
                            }
                        }))
                        .unwrap(),
                    ),
                }
                .to_der()
                .unwrap()
                .as_slice(),
            )
            .unwrap(),
        ),
    };
    let mut sig = sig_data.to_der().unwrap();
    // pad with 0 to keep the ranges correct
    sig.extend(std::iter::repeat(0).take(SIG_SIZE - sig.len()));
    let mut encoded_sig = Vec::with_capacity(sig.len() * 2);
    Str(&sig).write(&mut encoded_sig);

    dbg!(range.len(), encoded_sig.len());
    bytes.splice(range, encoded_sig);

    bytes
}
