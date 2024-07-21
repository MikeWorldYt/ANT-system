import { Component, ElementRef, HostListener, QueryList, ViewChildren, OnInit, AfterViewInit } from '@angular/core';
import { HeaderT02Component } from '../header-t02/header-t02.component';
import { CommonModule } from '@angular/common';
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { LightboxComponent } from '../../../features/lightbox/lightbox.component';
import { LanguageService } from '../../../services/lenguaje.service';
import { content } from '../../content/content';
import { Language } from '../../../services/language.types';
// import { en } from '../../content/content.json'

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule,

    LightboxComponent,
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component implements OnInit, AfterViewInit {
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService
  ) { }

  // For inner content
  write: any;

  // Language
  currentLanguage: Language = 'ES';

  ngOnInit(): void {
    // Initial content
    this.write = content[this.currentLanguage].title_02.module_01;

    // Suscribe to Language Service
    this.languageService.language$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_02.module_01;
        this.currentLanguage = language;
      }
    });
  }

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }

  switchLanguage(language: Language) {
    this.languageService.setLanguage(language);
  }
  

  // Hash Sections
  hovered = false;

  showHash(event: Event) {
    this.hovered = true;
  }
  
  hideHash(event: Event) {
    this.hovered = false;
  }

  // service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;
  
  idActive: string = '';

  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });

    setInterval(() => {
      this.idActive = this.intersectionService.getCurrentId();
      console.log(this.idActive);
    }, 100);
  }



}