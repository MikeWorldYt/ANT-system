import { Component, ElementRef, HostListener, QueryList, ViewChildren, OnInit, AfterViewInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { AddHyphenPipe } from '../../../pipes/add-hyphen.pipe';

// ▲ SERVICES ▲
import { ArticleService } from '../../../services/navArticleObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { PageService } from '../../../services/navPage.service';
import { Language } from '../../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../content/content';
import { LightboxComponent } from '../../../features/lightbox/lightbox.component';

// Testing
import { HeaderT02Component } from '../header-t02/header-t02.component';

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule,
    RouterLink,
    LightboxComponent,
    AddHyphenPipe,
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private languageService: LanguageService,
    private pageService: PageService,
    private intersectionService: ArticleService,
  ) { }

  // ▬▬▬ For inner content
  write: any;

  // ▬▬▬ Navigation Context
  currentLanguage: Language = 'ES';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';

  // ████ OnInit ███
  ngOnInit(): void {
    // Initial content
    this.write = content[this.currentLanguage].title_02.page_01;

    // Subscribe to Language Service
    this.languageService.currentLanguage$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_02.page_01;
        this.currentLanguage = language;
      }
    });
  }


  // ███ Language Controller 
  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }

  
  // ████ Hash Sections 
  hovered = false;

  showHash(event: Event) {
    this.hovered = true;
  }
  
  hideHash(event: Event) {
    this.hovered = false;
  }

  // ▲ service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  // ████ AfterViewInit ████
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });

    // Intersection Observer function
    this.intersectionService.getcurrentArticle().subscribe(id => {
      this.currentArticle = id;
    })

  }

}