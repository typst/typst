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
    types::SigFlags, writers::Form, Date, Finish, Name, Pdf, Primitive, Ref, Str,
};
use rsa::{traits::SignatureScheme, Pkcs1v15Sign, RsaPrivateKey};
use sha2::Sha512;

use crate::{PdfChunk, WithGlobalRefs};

const SIG_SIZE: usize = 1024 * 4;

pub fn alloc_signature_annotation(_: &WithGlobalRefs) -> (PdfChunk, Ref) {
    let mut chunk = PdfChunk::new();
    let r = chunk.alloc();
    (chunk, r)
}

pub fn prepare(
    alloc: &mut Ref,
    pdf: &mut Pdf,
    signature_annotation_ref: Ref,
    last_page_ref: Ref,
) -> (Range<usize>, Ref) {
    let form_ref = alloc.bump();
    let field_lock_ref = alloc.bump();

    let mut lock = pdf.indirect(field_lock_ref).dict();
    lock.pair(Name(b"Type"), Name(b"SigFieldLock"));
    lock.pair(Name(b"Action"), Name(b"All"));
    lock.finish();

    let mut signature_field = pdf.indirect(signature_annotation_ref).dict();
    signature_field.pair(Name(b"Type"), Name(b"Annot"));
    signature_field.pair(Name(b"Subtype"), Name(b"Widget"));
    signature_field.pair(Name(b"FT"), Name(b"Sig"));
    signature_field.pair(Name(b"F"), 132);
    signature_field.pair(Name(b"T"), Str(b"Signature"));
    signature_field.pair(Name(b"P"), last_page_ref);
    signature_field.pair(Name(b"Lock"), field_lock_ref);
    signature_field
        .insert(Name(b"Rect"))
        .array()
        .items([0.0, 0.0, 0.0, 0.0]);
    let mut signature_dict = signature_field.insert(Name(b"V")).dict();
    signature_dict.pair(Name(b"Type"), Name(b"Sig"));
    signature_dict.pair(Name(b"Filter"), Name(b"Adobe.PPKLite"));
    signature_dict.pair(Name(b"SubFilter"), Name(b"adbe.pkcs7.detached"));
    signature_dict.pair(Name(b"Name"), Str(b"Ana Gelez"));
    signature_dict
        .pair(Name(b"M"), Date::new(2024).month(08).day(12).hour(15).minute(55));
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

    let mut sig_refs = signature_dict.insert(Name(b"Reference")).array();
    let mut sig_ref = sig_refs.push().dict();
    sig_ref.pair(Name(b"Type"), Name(b"SigRef"));
    sig_ref.pair(Name(b"TransformMethod"), Name(b"DocMDP"));
    let mut params = sig_ref.insert(Name(b"TransformParams")).dict();
    params.pair(Name(b"Type"), Name(b"TransformParams"));
    params.pair(Name(b"P"), 1);
    params.finish();
    sig_ref.pair(Name(b"DigestMethod"), Name(b"SHA1"));
    sig_ref.finish();
    sig_refs.finish();

    signature_dict.finish();
    signature_field.finish();

    let mut form: Form = pdf.indirect(form_ref).start();
    form.fields([signature_annotation_ref]);
    form.sig_flags(SigFlags::SIGNATURES_EXIST | SigFlags::APPEND_ONLY);

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
    <i32 as pdf_writer::Primitive>::write(
        (bytes.len() - range.end) as i32,
        &mut actual_size,
    );
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
